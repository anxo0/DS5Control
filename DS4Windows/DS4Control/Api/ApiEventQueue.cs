using System;
using System.Threading.Channels;
using System.Threading.Tasks;

namespace DS4Windows.Api
{
    /// <summary>
    /// Runs API notifications in order on one background task. Posting never blocks, so it is safe
    /// from an input thread. An input thread only posts on real changes, never per report.
    /// </summary>
    public sealed class ApiEventQueue : IDisposable
    {
        private readonly Channel<Action> channel = Channel.CreateUnbounded<Action>(
            new UnboundedChannelOptions { SingleReader = true, AllowSynchronousContinuations = false });

        public ApiEventQueue()
        {
            Task.Run(ReadLoop);
        }

        public void Post(Action action)
        {
            channel.Writer.TryWrite(action);
        }

        private async Task ReadLoop()
        {
            await foreach (Action action in channel.Reader.ReadAllAsync().ConfigureAwait(false))
            {
                try
                {
                    action();
                }
                catch (Exception e)
                {
                    AppLogger.LogToGui($"Controller API subscriber failed: {e.Message}", true);
                }
            }
        }

        public void Dispose()
        {
            channel.Writer.TryComplete();
        }
    }
}
