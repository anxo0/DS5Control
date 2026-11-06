using System.Drawing;
using DS4Windows;

namespace DS4WindowsTests
{
    [TestClass]
    public class WheelAngleTests
    {
        private static readonly Point Origin = new Point(0, 0);

        private static int Angle(int accelX, int accelZ, Point center, Point circleCenter)
        {
            return Mapping.CalculateControllerAngle(accelX, accelZ, center, circleCenter, circleCenter);
        }

        [TestMethod]
        [DataRow(0, 101, 0, DisplayName = "Close enough to the center point")]
        [DataRow(100, 0, 900, DisplayName = "90 degrees right")]
        [DataRow(-100, 0, -900, DisplayName = "90 degrees left")]
        [DataRow(100, 100, 450, DisplayName = "45 degrees right")]
        [DataRow(-100, 100, -450, DisplayName = "45 degrees left")]
        public void ValidValuesKeepTheirAngle(int accelX, int accelZ, int expected)
        {
            Assert.AreEqual(expected, Angle(accelX, accelZ, new Point(0, 100), Origin));
        }

        [TestMethod]
        public void LeftAndRightUseTheirOwnCalibrationCircle()
        {
            Point center = new Point(0, 100);
            Point circleRight = new Point(0, 0);
            Point circleLeft = new Point(0, 50);

            Assert.AreEqual(900, Mapping.CalculateControllerAngle(100, 0, center, circleRight, circleLeft));
            Assert.AreEqual(-900, Mapping.CalculateControllerAngle(-100, 50, center, circleRight, circleLeft));
        }

        [TestMethod]
        public void ParallelVectorsWithCosineAboveOneReturnZero()
        {
            // Rounding gives a cosine of 1.0000000000000002 for these vectors
            Assert.AreEqual(0, Angle(2, 10, new Point(1, 5), Origin));
        }

        [TestMethod]
        public void OppositeVectorsWithCosineBelowMinusOneReturnFullTurn()
        {
            // Rounding gives a cosine of -1.0000000000000002 for these vectors
            Assert.AreEqual(-1800, Angle(-1, -5, new Point(1, 5), Origin));
        }

        [TestMethod]
        public void LargeSensorValuesDoNotOverflowTheVectorMath()
        {
            // 40000 * 40000 * 2 does not fit in an int
            Assert.AreEqual(450, Angle(40000, 40000, new Point(0, 100), Origin));
        }

        [TestMethod]
        public void MissingCalibrationReturnsZero()
        {
            Assert.AreEqual(0, Angle(5, 5, Origin, Origin));
        }

        [TestMethod]
        public void TiltAtCircleCenterReturnsZero()
        {
            Assert.AreEqual(0, Angle(10, 0, new Point(10, 10), new Point(10, 0)));
        }

        [TestMethod]
        public void SensorRangeSweepStaysInsideTheAngleRange()
        {
            int[] samples = { short.MinValue, -20000, -8192, -5, -1, 0, 1, 5, 2, 10, 8192, 20000, short.MaxValue };
            foreach (int centerX in samples)
            foreach (int centerZ in samples)
            foreach (int circleZ in samples)
            foreach (int accelX in samples)
            foreach (int accelZ in samples)
            {
                int result = Angle(accelX, accelZ, new Point(centerX, centerZ), new Point(centerX, circleZ));
                Assert.IsTrue(result >= -1800 && result <= 1800, $"Angle out of range: {result}");
            }
        }
    }
}
