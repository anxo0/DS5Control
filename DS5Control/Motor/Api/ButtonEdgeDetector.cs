using System.Runtime.CompilerServices;

namespace DS5Control.Api
{
    /// <summary>
    /// Detects the moment a button goes from released to pressed. It is meant to run once per
    /// input report, so it stays a plain struct: one comparison, no locks and no allocations.
    /// </summary>
    public struct ButtonEdgeDetector
    {
        private bool previous;

        /// <summary>Feeds the current button state.</summary>
        /// <returns>True only for the report in which the button became pressed.</returns>
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public bool Update(bool pressed)
        {
            bool isRisingEdge = pressed && !previous;
            previous = pressed;
            return isRisingEdge;
        }

        public void Reset()
        {
            previous = false;
        }
    }
}
