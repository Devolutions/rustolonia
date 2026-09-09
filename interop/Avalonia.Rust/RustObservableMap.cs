using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Specialized;
using System.ComponentModel;

namespace Avalonia.Rust;

/// <summary>
/// An observable keyed map projected from Rust.
///
/// Insert, replace, remove and clear are published incrementally: nothing is
/// rematerialized wholesale, and a replace of one key raises exactly one
/// <see cref="NotifyCollectionChangedAction.Replace"/>. Insertion order is
/// preserved and stable so those index-carrying notifications are meaningful
/// to an <c>ItemsControl</c> bound straight at the map.
///
/// It enumerates as <see cref="KeyValuePair{TKey,TValue}"/>, which is what
/// compiled AXAML binds against, and it owns nested-model values exactly like
/// a nested collection: a displaced or removed value that is
/// <see cref="IDisposable"/> is detached.
/// </summary>
public sealed class RustObservableMap<TKey, TValue> :
    IReadOnlyDictionary<TKey, TValue>,
    IReadOnlyList<KeyValuePair<TKey, TValue>>,
    IEnumerable<KeyValuePair<TKey, TValue>>,
    IList,
    INotifyCollectionChanged,
    INotifyPropertyChanged
    where TKey : notnull
{
    private readonly Dictionary<TKey, int> _indices;
    private readonly List<KeyValuePair<TKey, TValue>> _entries = [];

    public RustObservableMap()
        : this(EqualityComparer<TKey>.Default)
    {
    }

    public RustObservableMap(IEqualityComparer<TKey> comparer)
    {
        ArgumentNullException.ThrowIfNull(comparer);
        _indices = new Dictionary<TKey, int>(comparer);
    }

    public event NotifyCollectionChangedEventHandler? CollectionChanged;
    public event PropertyChangedEventHandler? PropertyChanged;

    public int Count => _entries.Count;

    public TValue this[TKey key] => _entries[_indices[key]].Value;

    public KeyValuePair<TKey, TValue> this[int index] => _entries[index];

    public IEnumerable<TKey> Keys
    {
        get
        {
            foreach (var entry in _entries)
                yield return entry.Key;
        }
    }

    public IEnumerable<TValue> Values
    {
        get
        {
            foreach (var entry in _entries)
                yield return entry.Value;
        }
    }

    public bool ContainsKey(TKey key) => _indices.ContainsKey(key);

    public bool TryGetValue(TKey key, out TValue value)
    {
        if (_indices.TryGetValue(key, out var index))
        {
            value = _entries[index].Value;
            return true;
        }
        value = default!;
        return false;
    }

    /// <summary>
    /// Inserts or replaces one key. <paramref name="displaced"/> receives the
    /// previous value when a replace displaced one, so the caller decides when
    /// to detach it (after notifications, never during a commit).
    /// </summary>
    public bool Set(TKey key, TValue value, out TValue? displaced)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (_indices.TryGetValue(key, out var index))
        {
            var previous = _entries[index];
            if (EqualityComparer<TValue>.Default.Equals(previous.Value, value))
            {
                displaced = default;
                return false;
            }
            _entries[index] = new KeyValuePair<TKey, TValue>(key, value);
            displaced = previous.Value;
            RaisePropertyChanged("Item[]");
            CollectionChanged?.Invoke(
                this,
                new NotifyCollectionChangedEventArgs(
                    NotifyCollectionChangedAction.Replace,
                    _entries[index],
                    previous,
                    index));
            return true;
        }

        displaced = default;
        var appended = new KeyValuePair<TKey, TValue>(key, value);
        _indices[key] = _entries.Count;
        _entries.Add(appended);
        RaisePropertyChanged(nameof(Count));
        RaisePropertyChanged("Item[]");
        CollectionChanged?.Invoke(
            this,
            new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Add, appended, _entries.Count - 1));
        return true;
    }

    /// <summary>Removes one key. A missing key is a no-op, not an error.</summary>
    public bool Remove(TKey key, out TValue? removed)
    {
        ArgumentNullException.ThrowIfNull(key);
        if (!_indices.Remove(key, out var index))
        {
            removed = default;
            return false;
        }
        var entry = _entries[index];
        _entries.RemoveAt(index);
        for (var i = index; i < _entries.Count; i++)
            _indices[_entries[i].Key] = i;
        removed = entry.Value;
        RaisePropertyChanged(nameof(Count));
        RaisePropertyChanged("Item[]");
        CollectionChanged?.Invoke(
            this,
            new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Remove, entry, index));
        return true;
    }

    /// <summary>Clears the map and hands back every removed value for detachment.</summary>
    public IReadOnlyList<TValue> Clear()
    {
        if (_entries.Count == 0)
            return [];
        var removed = new List<TValue>(_entries.Count);
        foreach (var entry in _entries)
            removed.Add(entry.Value);
        _entries.Clear();
        _indices.Clear();
        RaisePropertyChanged(nameof(Count));
        RaisePropertyChanged("Item[]");
        CollectionChanged?.Invoke(this, new NotifyCollectionChangedEventArgs(NotifyCollectionChangedAction.Reset));
        return removed;
    }

    public IEnumerator<KeyValuePair<TKey, TValue>> GetEnumerator() => _entries.GetEnumerator();

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    // The non-generic IList surface exists so an ItemsControl can bind
    // straight at the map: Avalonia's ItemsSourceView rejects a source that
    // raises collection notifications without also being an IList, because it
    // needs indexed access to translate them.
    bool IList.IsFixedSize => false;

    bool IList.IsReadOnly => true;

    bool ICollection.IsSynchronized => false;

    object ICollection.SyncRoot => this;

    object? IList.this[int index]
    {
        get => _entries[index];
        set => throw new NotSupportedException("A Rust-owned map is read-only from managed code.");
    }

    int IList.Add(object? value) => throw new NotSupportedException("A Rust-owned map is read-only from managed code.");

    void IList.Clear() => throw new NotSupportedException("A Rust-owned map is read-only from managed code.");

    void IList.Insert(int index, object? value) => throw new NotSupportedException("A Rust-owned map is read-only from managed code.");

    void IList.Remove(object? value) => throw new NotSupportedException("A Rust-owned map is read-only from managed code.");

    void IList.RemoveAt(int index) => throw new NotSupportedException("A Rust-owned map is read-only from managed code.");

    bool IList.Contains(object? value) =>
        value is KeyValuePair<TKey, TValue> entry && _indices.ContainsKey(entry.Key);

    int IList.IndexOf(object? value) =>
        value is KeyValuePair<TKey, TValue> entry && _indices.TryGetValue(entry.Key, out var index) &&
        EqualityComparer<TValue>.Default.Equals(_entries[index].Value, entry.Value)
            ? index
            : -1;

    void ICollection.CopyTo(Array array, int index)
    {
        ArgumentNullException.ThrowIfNull(array);
        for (var i = 0; i < _entries.Count; i++)
            array.SetValue(_entries[i], index + i);
    }

    private void RaisePropertyChanged(string name) =>
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}
