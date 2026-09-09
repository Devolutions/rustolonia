using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Specialized;
using System.ComponentModel;
using Avalonia.Rust.Interop;

namespace Avalonia.Rust;

/// <summary>
/// Creates the managed presentation object for one realized element of a
/// windowed collection. Nested-model windows hand back a generated adapter
/// (which is <see cref="IDisposable"/> and is detached on eviction); string
/// windows hand back the decoded text.
/// </summary>
/// <param name="model">The element's nested view model, or null for a string element.</param>
/// <param name="text">The element's decoded text, or null for a model element.</param>
public delegate object RustWindowElementFactory(IAvnRustViewModel? model, string? text);

/// <summary>
/// A range-backed projection of a Rust-owned dataset.
///
/// The point of this type is that <see cref="Count"/> is the dataset's total
/// size while the number of live managed element objects is bounded by
/// <c>pageSize * maxLivePages</c>. Rust owns the rows; managed code realizes
/// only the pages the presentation actually indexes, and the least recently
/// used page beyond the budget is evicted and its adapters detached.
///
/// It implements the non-generic <see cref="IList"/> because that is what
/// Avalonia's <c>ItemsSourceView</c> uses to read <c>Count</c> and index
/// items without enumerating; enumeration here deliberately never triggers
/// realization, so an accidental <c>foreach</c> cannot materialize the whole
/// dataset.
///
/// Every public member is UI-thread affine. Range results arrive through
/// <see cref="RustRangeCoordinator"/>, which posts decoding to the dispatcher
/// and never touches this object on a Rust worker stack.
/// </summary>
public sealed class RustWindowedCollection : IList, IReadOnlyList<object?>, INotifyCollectionChanged, INotifyPropertyChanged, IDisposable, Avalonia.Controls.IViewportRangeSource
{
    private readonly Dictionary<int, object?[]> _pages = [];
    private readonly LinkedList<int> _recent = new();
    private readonly Dictionary<int, LinkedListNode<int>> _recentNodes = [];
    private readonly HashSet<int> _pending = [];
    private readonly RustWindowElementFactory _factory;
    private IAvnRustRangeSource? _source;
    private Func<int, RustWindowedCollection?>? _resolvePeer;
    private long _generation;
    private int _totalCount;
    private int _liveElements;
    private bool _disposed;

    /// <param name="collectionId">The schema collection ID this window projects.</param>
    /// <param name="pageSize">Number of elements realized per range request.</param>
    /// <param name="maxLivePages">Maximum number of realized pages retained.</param>
    /// <param name="factory">Builds one managed element from its transported form.</param>
    public RustWindowedCollection(int collectionId, int pageSize, int maxLivePages, RustWindowElementFactory factory)
    {
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(pageSize);
        ArgumentOutOfRangeException.ThrowIfNegativeOrZero(maxLivePages);
        ArgumentNullException.ThrowIfNull(factory);
        CollectionId = collectionId;
        PageSize = pageSize;
        MaxLivePages = maxLivePages;
        _factory = factory;
    }

    public event NotifyCollectionChangedEventHandler? CollectionChanged;
    public event PropertyChangedEventHandler? PropertyChanged;

    public int CollectionId { get; }

    public int PageSize { get; }

    public int MaxLivePages { get; }

    /// <summary>The dataset generation this window is currently realizing against.</summary>
    public long Generation => _generation;

    /// <summary>Number of realized (live) element objects. Bounded by <c>PageSize * MaxLivePages</c>.</summary>
    public int LiveElementCount => _liveElements;

    /// <summary>Number of element objects detached and released since construction.</summary>
    public int DetachedElementCount { get; private set; }

    /// <summary>Number of range requests posted to Rust since construction.</summary>
    public int RangeRequestCount { get; private set; }

    /// <summary>Index of the first element of the most recently realized page, or -1.</summary>
    public int ViewportStart { get; private set; } = -1;

    /// <summary>The dataset's total element count, which is also <see cref="Count"/>.</summary>
    public int TotalCount => _totalCount;

    public int Count => _totalCount;

    bool IList.IsFixedSize => true;

    bool IList.IsReadOnly => true;

    bool ICollection.IsSynchronized => false;

    object ICollection.SyncRoot => this;

    object? IList.this[int index]
    {
        get => this[index];
        set => throw new NotSupportedException("A Rust-owned windowed collection is read-only.");
    }

    /// <summary>
    /// Returns the realized element, or null while its page is being fetched.
    /// A miss posts one nonblocking range request; it never blocks the caller
    /// and never enters Rust re-entrantly with a result.
    /// </summary>
    public object? this[int index]
    {
        get
        {
            if ((uint)index >= (uint)_totalCount)
                throw new ArgumentOutOfRangeException(nameof(index));
            var page = index / PageSize;
            if (_pages.TryGetValue(page, out var items))
            {
                Touch(page);
                return items[index - (page * PageSize)];
            }
            RequestPage(page);
            return null;
        }
    }

    /// <summary>
    /// Binds the Rust-implemented range capability. Called once, right after
    /// the owning adapter attaches, because the capability is queried from the
    /// view model rather than handed to the constructor.
    /// </summary>
    public void SetSource(IAvnRustRangeSource? source) => _source = source;

    /// <summary>
    /// Resolves other windowed collections on the same model. The range-request
    /// queue is shared and bounded, so accepting this window's request may
    /// evict another collection's; that victim must stop waiting for a batch
    /// that will never arrive.
    /// </summary>
    public void SetPeerResolver(Func<int, RustWindowedCollection?>? resolve) => _resolvePeer = resolve;

    /// <summary>
    /// Republishes the dataset identity. A new generation invalidates every
    /// realized page (its rows may no longer be at the same index), so all
    /// live elements are detached and one Reset is raised.
    /// </summary>
    public void ResetTo(long generation, long totalCount)
    {
        if (_disposed)
            return;
        var total = ClampTotal(totalCount);
        if (generation == _generation && total == _totalCount)
            return;
        _generation = generation;
        _totalCount = total;
        _pending.Clear();
        ViewportStart = -1;
        DropAllPages();
        RaiseReset();
    }

    /// <summary>
    /// Applies one realized range. Returns false when the batch is stale, in
    /// which case nothing is realized and nothing is notified.
    /// </summary>
    public bool ApplyRange(long generation, long totalCount, long offset, IReadOnlyList<object?> elements)
    {
        ArgumentNullException.ThrowIfNull(elements);
        if (_disposed)
            return false;
        if (generation != _generation)
            return false;
        AbandonPage(generation, offset);
        var total = ClampTotal(totalCount);
        if (total != _totalCount)
            return false;
        if (offset < 0 || offset % PageSize != 0 || offset >= _totalCount)
            return false;
        var page = (int)(offset / PageSize);
        var expected = Math.Min(PageSize, _totalCount - (page * PageSize));
        if (elements.Count != expected)
            return false;

        var items = new object?[expected];
        for (var i = 0; i < expected; i++)
            items[i] = elements[i];
        if (_pages.TryGetValue(page, out var previous))
            ReleasePage(previous);
        else
            _recentNodes[page] = _recent.AddLast(page);
        _pages[page] = items;
        _liveElements += expected;
        ViewportStart = page * PageSize;
        Touch(page);
        Evict();

        RaisePropertyChanged("Item[]");
        RaiseDiagnosticsChanged();
        var start = page * PageSize;
        for (var i = 0; i < expected; i++)
        {
            CollectionChanged?.Invoke(
                this,
                new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Replace, items[i], null, start + i));
        }
        return true;
    }

    /// <summary>
    /// Requests every page overlapping <paramref name="firstIndex"/>..
    /// <paramref name="lastIndex"/> (inclusive). Called from
    /// <see cref="Avalonia.Controls.TableView"/> after layout so the
    /// viewport, not stray indexer misses, drives realization.
    /// </summary>
    public void EnsureVisibleRange(int firstIndex, int lastIndex)
    {
        if (_disposed || _source is null || _totalCount <= 0)
            return;
        if (firstIndex < 0)
            firstIndex = 0;
        if (lastIndex >= _totalCount)
            lastIndex = _totalCount - 1;
        if (lastIndex < firstIndex)
            return;
        var firstPage = firstIndex / PageSize;
        var lastPage = lastIndex / PageSize;
        for (var page = firstPage; page <= lastPage; page++)
        {
            if (!_pages.ContainsKey(page))
                RequestPage(page);
        }
    }

    /// <summary>
    /// Re-requests every currently realized page at the current generation.
    /// Used by a live dataset (process CPU%, log tail of unchanged identity)
    /// so values update in place without <see cref="ResetTo"/> churning
    /// adapters, scroll, or selection. Unrealized pages are left alone.
    /// </summary>
    public void RefreshRealized()
    {
        if (_disposed || _source is null)
            return;
        var pages = new int[_pages.Count];
        _pages.Keys.CopyTo(pages, 0);
        foreach (var page in pages)
        {
            _pending.Remove(page);
            RequestPage(page);
        }
    }

    /// <summary>
    /// Marks a page as no longer outstanding without realizing it. Called for
    /// every terminal outcome that is not an applied fill (a decode failure, a
    /// stale generation, or a request Rust's bounded queue evicted), so a page
    /// is never stranded waiting for a batch that will not arrive.
    /// </summary>
    public void AbandonPage(long offset)
    {
        if (offset < 0 || offset % PageSize != 0)
            return;
        _pending.Remove((int)(offset / PageSize));
    }

    /// <summary>Abandons only a request belonging to the supplied dataset generation.</summary>
    public void AbandonPage(long generation, long offset)
    {
        if (generation == _generation)
            AbandonPage(offset);
    }

    /// <summary>
    /// Builds one element from its transported form. Used by the range
    /// coordinator while decoding, before anything is committed.
    /// </summary>
    public object CreateElement(IAvnRustViewModel? model, string? text) => _factory(model, text);

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;
        _pending.Clear();
        _source = null;
        DropAllPages();
        _totalCount = 0;
        // A still-bound ItemsControl caches the old count; without this Reset
        // its next indexed read would fall outside the (now empty) range.
        RaiseReset();
    }

    int IList.Add(object? value) => throw new NotSupportedException("A Rust-owned windowed collection is read-only.");

    void IList.Clear() => throw new NotSupportedException("A Rust-owned windowed collection is read-only.");

    void IList.Insert(int index, object? value) => throw new NotSupportedException("A Rust-owned windowed collection is read-only.");

    void IList.Remove(object? value) => throw new NotSupportedException("A Rust-owned windowed collection is read-only.");

    void IList.RemoveAt(int index) => throw new NotSupportedException("A Rust-owned windowed collection is read-only.");

    /// <summary>Searches realized pages only; an unrealized element reports -1.</summary>
    public int IndexOf(object? value)
    {
        foreach (var (page, items) in _pages)
        {
            for (var i = 0; i < items.Length; i++)
            {
                if (Equals(items[i], value))
                    return (page * PageSize) + i;
            }
        }
        return -1;
    }

    bool IList.Contains(object? value) => IndexOf(value) >= 0;

    void ICollection.CopyTo(Array array, int index)
    {
        ArgumentNullException.ThrowIfNull(array);
        for (var i = 0; i < _totalCount; i++)
            array.SetValue(Realized(i), index + i);
    }

    /// <summary>
    /// Enumerates realized elements and placeholders without realizing
    /// anything: a stray enumeration of a 100K dataset must not allocate 100K
    /// adapters.
    /// </summary>
    public IEnumerator<object?> GetEnumerator()
    {
        for (var i = 0; i < _totalCount; i++)
            yield return Realized(i);
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    private object? Realized(int index)
    {
        var page = index / PageSize;
        return _pages.TryGetValue(page, out var items) ? items[index - (page * PageSize)] : null;
    }

    private void RequestPage(int page)
    {
        if (_disposed || _source is null || !_pending.Add(page))
            return;
        var offset = (long)page * PageSize;
        var length = (int)Math.Min(PageSize, _totalCount - offset);
        if (length <= 0)
        {
            _pending.Remove(page);
            return;
        }
        RangeRequestCount++;
        try
        {
            var hr = _source.RequestRange(
                CollectionId, offset, length, _generation, out var droppedCollection, out var dropped);
            if (hr < 0)
                _pending.Remove(page);
            else if (dropped >= 0)
            {
                if (droppedCollection == CollectionId)
                    AbandonPage(dropped);
                else
                    _resolvePeer?.Invoke(droppedCollection)?.AbandonPage(dropped);
            }
        }
        catch
        {
            // Rust owns the request queue; a failed request simply leaves the
            // page unrealized so a later index can ask again.
            _pending.Remove(page);
        }
    }

    private void Touch(int page)
    {
        if (!_recentNodes.TryGetValue(page, out var node))
            return;
        _recent.Remove(node);
        _recent.AddLast(node);
    }

    private void Evict()
    {
        while (_pages.Count > MaxLivePages && _recent.First is { } oldest)
        {
            var page = oldest.Value;
            _recent.RemoveFirst();
            _recentNodes.Remove(page);
            if (_pages.Remove(page, out var items))
                ReleasePage(items);
        }
    }

    private void DropAllPages()
    {
        foreach (var items in _pages.Values)
            ReleasePage(items);
        _pages.Clear();
        _recent.Clear();
        _recentNodes.Clear();
    }

    private void ReleasePage(object?[] items)
    {
        foreach (var item in items)
        {
            if (item is IDisposable disposable)
            {
                try { disposable.Dispose(); }
                catch { /* A detaching element must never break eviction. */ }
                DetachedElementCount++;
            }
        }
        _liveElements -= items.Length;
        if (_liveElements < 0)
            _liveElements = 0;
    }

    private int ClampTotal(long totalCount) =>
        totalCount <= 0 ? 0 : totalCount > int.MaxValue ? int.MaxValue : (int)totalCount;

    private void RaiseReset()
    {
        RaisePropertyChanged(nameof(Count));
        RaisePropertyChanged("Item[]");
        RaiseDiagnosticsChanged();
        CollectionChanged?.Invoke(this, new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset));
    }

    /// <summary>
    /// Publishes the realization counters. They are the evidence that the
    /// window is actually bounded, so a presentation that shows them must see
    /// them change rather than reading a stale first-frame zero.
    /// </summary>
    private void RaiseDiagnosticsChanged()
    {
        RaisePropertyChanged(nameof(LiveElementCount));
        RaisePropertyChanged(nameof(DetachedElementCount));
        RaisePropertyChanged(nameof(ViewportStart));
    }

    private void RaisePropertyChanged(string name) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
