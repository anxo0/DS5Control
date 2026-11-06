using System;

namespace DS5Control.Api
{
    /// <summary>
    /// Presents the single profile colour as colour plus brightness. Profiles have no brightness
    /// field: the colour they store already includes it, so the brightness is the strength of its
    /// brightest channel. This class remembers the unscaled colour between changes, so lowering the
    /// brightness and raising it again, even through 0, returns to the same colour. A colour
    /// changed from outside is picked up again by <see cref="Sync"/>.
    /// </summary>
    public sealed class LightLevel
    {
        private const int FullBrightness = 100;

        private DS5Color baseColor = new DS5Color(255, 255, 255);
        private DS5Color lastOutput;
        private bool hasOutput;

        public DS5Color BaseColor => baseColor;
        public int Brightness { get; private set; } = FullBrightness;

        /// <summary>Aligns the state with the colour stored in the profile.</summary>
        public void Sync(DS5Color storedColor)
        {
            if (hasOutput && storedColor.Equals(lastOutput)) return;

            int strongest = Math.Max(storedColor.red, Math.Max(storedColor.green, storedColor.blue));
            if (strongest > 0)
            {
                baseColor = Scale(storedColor, 255.0 / strongest);
            }

            Brightness = (int)Math.Round(strongest * (double)FullBrightness / 255.0);
            lastOutput = storedColor;
            hasOutput = true;
        }

        /// <returns>The colour to store in the profile.</returns>
        public DS5Color SetColor(DS5Color color)
        {
            baseColor = color;
            return Apply();
        }

        /// <returns>The colour to store in the profile.</returns>
        public DS5Color SetBrightness(int percent)
        {
            Brightness = percent;
            return Apply();
        }

        private DS5Color Apply()
        {
            lastOutput = Scale(baseColor, Brightness / (double)FullBrightness);
            hasOutput = true;
            return lastOutput;
        }

        private static DS5Color Scale(DS5Color color, double factor)
        {
            return new DS5Color(ScaleChannel(color.red, factor), ScaleChannel(color.green, factor),
                ScaleChannel(color.blue, factor));
        }

        private static byte ScaleChannel(byte channel, double factor)
        {
            return (byte)Math.Min(255, Math.Round(channel * factor));
        }
    }
}
