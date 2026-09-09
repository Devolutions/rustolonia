using System.Threading;
using System.Threading.Tasks;
using Avalonia.Host.Com;
using Xunit;

namespace Avalonia.Host.Tests;

public class AsyncOperationTests
{
    [Fact]
    public void Completion_reports_success_exactly_once()
    {
        var operations = new AvnAsyncOperations();
        var completion = new Completion();

        Assert.Equal(
            0,
            operations.Start(
                completion,
                async cancellation =>
                {
                    await Task.Delay(1, cancellation);
                    return AvnAsyncValue.FromString("done");
                },
                out var operationId));

        var result = completion.Wait();
        Assert.Equal(operationId, result.OperationId);
        Assert.Equal(0, result.HResult);
        Assert.Equal(4, result.ValueKind);
        Assert.Equal("done", result.StringValue);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void Cancellation_completes_with_abort()
    {
        var operations = new AvnAsyncOperations();
        var completion = new Completion();
        Assert.Equal(
            0,
            operations.Start(
                completion,
                async cancellation =>
                {
                    await Task.Delay(30_000, cancellation);
                    return AvnAsyncValue.None;
                },
                out var operationId));

        Assert.Equal(0, operations.Cancel(operationId));

        var result = completion.Wait();
        Assert.Equal(operationId, result.OperationId);
        Assert.Equal(unchecked((int)0x80004004), result.HResult);
        Assert.Equal(1, completion.CallCount);
    }

    [Fact]
    public void Failing_completion_callback_is_not_invoked_recursively()
    {
        var operations = new AvnAsyncOperations();
        var completion = new FailingCompletion();

        Assert.Equal(
            0,
            operations.Start(
                completion,
                _ => Task.FromResult(AvnAsyncValue.None),
                out _));

        completion.Wait();
        Assert.Equal(1, completion.CallCount);
    }

    private sealed class Completion : IAvnAsyncCompletion
    {
        private readonly ManualResetEventSlim _completed = new();
        private CompletionResult? _result;

        public int CallCount { get; private set; }

        public CompletionResult Wait()
        {
            Assert.True(_completed.Wait(5_000));
            return Assert.IsType<CompletionResult>(_result);
        }

        public int Complete(
            long operationId,
            int hresult,
            int valueKind,
            long integerValue,
            double doubleValue,
            string? stringValue,
            string? error)
        {
            CallCount++;
            if (Interlocked.CompareExchange(
                    ref _result,
                    new CompletionResult(
                operationId,
                hresult,
                valueKind,
                integerValue,
                doubleValue,
                stringValue,
                        error),
                    null) is not null)
            {
                return unchecked((int)0x80004005);
            }
            _completed.Set();
            return 0;
        }
    }

    private sealed record CompletionResult(
        long OperationId,
        int HResult,
        int ValueKind,
        long IntegerValue,
        double DoubleValue,
        string? StringValue,
        string? Error);

    private sealed class FailingCompletion : IAvnAsyncCompletion
    {
        private readonly ManualResetEventSlim _called = new();

        public int CallCount { get; private set; }

        public void Wait() => Assert.True(_called.Wait(5_000));

        public int Complete(
            long operationId,
            int hresult,
            int valueKind,
            long integerValue,
            double doubleValue,
            string? stringValue,
            string? error)
        {
            CallCount++;
            _called.Set();
            return unchecked((int)0x80004005);
        }
    }
}
