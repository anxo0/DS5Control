/*
DS4Windows
Copyright (C) 2023  Travis Nickles

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

namespace DS5Control
{
    public class DS5StateFieldMapping
    {
        public enum ControlType : int { Unknown = 0, Button, AxisDir, Trigger, Touch, GyroDir, SwipeDir }
        public const byte LAST_DS5_ACTION = (byte)DS5Controls.TouchEnded;

        public bool[] buttons = new bool[(int)LAST_DS5_ACTION + 1];
        public byte[] axisdirs = new byte[(int)LAST_DS5_ACTION + 1];
        public byte[] triggers = new byte[(int)LAST_DS5_ACTION + 1];
        public int[] gryodirs = new int[(int)LAST_DS5_ACTION + 1];
        public byte[] swipedirs = new byte[(int)LAST_DS5_ACTION + 1];
        public bool[] swipedirbools = new bool[(int)LAST_DS5_ACTION + 1];
        public bool touchButton = false;
        public bool outputTouchButton = false;

        public static ControlType[] mappedType = new ControlType[LAST_DS5_ACTION + 1]
        {
            ControlType.Unknown, // DS5Controls.None
            ControlType.AxisDir, // DS5Controls.LXNeg
            ControlType.AxisDir, // DS5Controls.LXPos
            ControlType.AxisDir, // DS5Controls.LYNeg
            ControlType.AxisDir, // DS5Controls.LYPos
            ControlType.AxisDir, // DS5Controls.RXNeg
            ControlType.AxisDir, // DS5Controls.RXPos
            ControlType.AxisDir, // DS5Controls.RYNeg
            ControlType.AxisDir, // DS5Controls.RYPos
            ControlType.Button, // DS5Controls.L1
            ControlType.Trigger, // DS5Controls.L2
            ControlType.Button, // DS5Controls.L3
            ControlType.Button, // DS5Controls.R1
            ControlType.Trigger, // DS5Controls.R2
            ControlType.Button, // DS5Controls.R3
            ControlType.Button, // DS5Controls.Square
            ControlType.Button, // DS5Controls.Triangle
            ControlType.Button, // DS5Controls.Circle
            ControlType.Button, // DS5Controls.Cross
            ControlType.Button, // DS5Controls.DpadUp
            ControlType.Button, // DS5Controls.DpadRight
            ControlType.Button, // DS5Controls.DpadDown
            ControlType.Button, // DS5Controls.DpadLeft
            ControlType.Button, // DS5Controls.PS
            ControlType.Touch, // DS5Controls.TouchLeft
            ControlType.Touch, // DS5Controls.TouchUpper
            ControlType.Touch, // DS5Controls.TouchMulti
            ControlType.Touch, // DS5Controls.TouchRight
            ControlType.Button, // DS5Controls.Share
            ControlType.Button, // DS5Controls.Options
            ControlType.Button, // DS5Controls.Mute
            ControlType.Button, // DS5Controls.FnL
            ControlType.Button, // DS5Controls.FnR
            ControlType.Button, // DS5Controls.BLP
            ControlType.Button, // DS5Controls.BRP
            ControlType.GyroDir, // DS5Controls.GyroXPos
            ControlType.GyroDir, // DS5Controls.GyroXNeg
            ControlType.GyroDir, // DS5Controls.GyroZPos
            ControlType.GyroDir, // DS5Controls.GyroZNeg
            ControlType.SwipeDir, // DS5Controls.SwipeLeft
            ControlType.SwipeDir, // DS5Controls.SwipeRight
            ControlType.SwipeDir, // DS5Controls.SwipeUp
            ControlType.SwipeDir, // DS5Controls.SwipeDown
            ControlType.Button, // DS5Controls.L2FullPull
            ControlType.Button, // DS5Controls.R2FullPull
            ControlType.Button, // DS5Controls.GyroSwipeLeft
            ControlType.Button, // DS5Controls.GyroSwipeRight
            ControlType.Button, // DS5Controls.GyroSwipeUp
            ControlType.Button, // DS5Controls.GyroSwipeDown
            ControlType.Button, // DS5Controls.Capture
            ControlType.Button, // DS5Controls.SideL
            ControlType.Button, // DS5Controls.SideR
            ControlType.Trigger, // DS5Controls.LSOuter
            ControlType.Trigger, // DS5Controls.RSOuter
            ControlType.Button,  // DS5Controls.TouchStarted
            ControlType.Button, // DS5Controls.TouchEnded
        };

        public DS5StateFieldMapping()
        {
        }

        public DS5StateFieldMapping(DS5State cState, DS5StateExposed exposeState, Mouse tp, bool priorMouse = false)
        {
            PopulateFieldMapping(cState, exposeState, tp, priorMouse);
        }

        public void PopulateFieldMapping(DS5State cState, DS5StateExposed exposeState, Mouse tp, bool priorMouse = false)
        {
            unchecked
            {
                axisdirs[(int)DS5Controls.LXNeg] = cState.LX;
                axisdirs[(int)DS5Controls.LXPos] = cState.LX;
                axisdirs[(int)DS5Controls.LYNeg] = cState.LY;
                axisdirs[(int)DS5Controls.LYPos] = cState.LY;
                triggers[(int)DS5Controls.LSOuter] = cState.OutputLSOuter;

                axisdirs[(int)DS5Controls.RXNeg] = cState.RX;
                axisdirs[(int)DS5Controls.RXPos] = cState.RX;
                axisdirs[(int)DS5Controls.RYNeg] = cState.RY;
                axisdirs[(int)DS5Controls.RYPos] = cState.RY;
                triggers[(int)DS5Controls.RSOuter] = cState.OutputRSOuter;

                triggers[(int)DS5Controls.L2] = cState.L2;
                triggers[(int)DS5Controls.R2] = cState.R2;

                buttons[(int)DS5Controls.L1] = cState.L1;
                buttons[(int)DS5Controls.L2FullPull] = cState.L2Raw == 255;
                buttons[(int)DS5Controls.L3] = cState.L3;
                buttons[(int)DS5Controls.R1] = cState.R1;
                buttons[(int)DS5Controls.R2FullPull] = cState.R2Raw == 255;
                buttons[(int)DS5Controls.R3] = cState.R3;

                buttons[(int)DS5Controls.Cross] = cState.Cross;
                buttons[(int)DS5Controls.Triangle] = cState.Triangle;
                buttons[(int)DS5Controls.Circle] = cState.Circle;
                buttons[(int)DS5Controls.Square] = cState.Square;
                buttons[(int)DS5Controls.PS] = cState.PS;
                buttons[(int)DS5Controls.Options] = cState.Options;
                buttons[(int)DS5Controls.Share] = cState.Share;
                buttons[(int)DS5Controls.Mute] = cState.Mute;
                buttons[(int)DS5Controls.FnL] = cState.FnL;
                buttons[(int)DS5Controls.FnR] = cState.FnR;
                buttons[(int)DS5Controls.BLP] = cState.BLP;
                buttons[(int)DS5Controls.BRP] = cState.BRP;
                buttons[(int)DS5Controls.Capture] = cState.Capture;
                buttons[(int)DS5Controls.SideL] = cState.SideL;
                buttons[(int)DS5Controls.SideR] = cState.SideR;

                buttons[(int)DS5Controls.DpadUp] = cState.DpadUp;
                buttons[(int)DS5Controls.DpadRight] = cState.DpadRight;
                buttons[(int)DS5Controls.DpadDown] = cState.DpadDown;
                buttons[(int)DS5Controls.DpadLeft] = cState.DpadLeft;

                buttons[(int)DS5Controls.TouchLeft] = tp != null ? (!priorMouse ? tp.leftDown : tp.priorLeftDown) : false;
                buttons[(int)DS5Controls.TouchRight] = tp != null ? (!priorMouse ? tp.rightDown : tp.priorRightDown) : false;
                buttons[(int)DS5Controls.TouchUpper] = tp != null ? (!priorMouse ? tp.upperDown : tp.priorUpperDown) : false;
                buttons[(int)DS5Controls.TouchMulti] = tp != null ? (!priorMouse ? tp.multiDown : tp.priorMultiDown) : false;

                int sixAxisX = -exposeState.getOutputAccelX();
                gryodirs[(int)DS5Controls.GyroXPos] = sixAxisX > 0 ? sixAxisX : 0;
                gryodirs[(int)DS5Controls.GyroXNeg] = sixAxisX < 0 ? sixAxisX : 0;

                int sixAxisZ = exposeState.getOutputAccelZ();
                gryodirs[(int)DS5Controls.GyroZPos] = sixAxisZ > 0 ? sixAxisZ : 0;
                gryodirs[(int)DS5Controls.GyroZNeg] = sixAxisZ < 0 ? sixAxisZ : 0;

                swipedirs[(int)DS5Controls.SwipeLeft] = tp != null ? (!priorMouse ? tp.swipeLeftB : tp.priorSwipeLeftB) : (byte)0;
                swipedirs[(int)DS5Controls.SwipeRight] = tp != null ? (!priorMouse ? tp.swipeRightB : tp.priorSwipeRightB) : (byte)0;
                swipedirs[(int)DS5Controls.SwipeUp] = tp != null ? (!priorMouse ? tp.swipeUpB : tp.priorSwipeUpB) : (byte)0;
                swipedirs[(int)DS5Controls.SwipeDown] = tp != null ? (!priorMouse ? tp.swipeDownB : tp.priorSwipeDownB) : (byte)0;

                swipedirbools[(int)DS5Controls.SwipeLeft] = tp != null ? (!priorMouse ? tp.swipeLeft : tp.priorSwipeLeft) : false;
                swipedirbools[(int)DS5Controls.SwipeRight] = tp != null ? (!priorMouse ? tp.swipeRight : tp.priorSwipeRight) : false;
                swipedirbools[(int)DS5Controls.SwipeUp] = tp != null ? (!priorMouse ? tp.swipeUp : tp.priorSwipeUp) : false;
                swipedirbools[(int)DS5Controls.SwipeDown] = tp != null ? (!priorMouse ? tp.swipeDown : tp.priorSwipeDown) : false;

                buttons[(int)DS5Controls.GyroSwipeLeft] = tp != null ? tp.gyroSwipe.swipeLeft : false;
                buttons[(int)DS5Controls.GyroSwipeRight] = tp != null ? tp.gyroSwipe.swipeRight : false;
                buttons[(int)DS5Controls.GyroSwipeUp] = tp != null ? tp.gyroSwipe.swipeUp : false;
                buttons[(int)DS5Controls.GyroSwipeDown] = tp != null ? tp.gyroSwipe.swipeDown : false;
                buttons[(int)DS5Controls.TouchStarted] = tp != null ? tp.TouchStarted : false;
                buttons[(int)DS5Controls.TouchEnded] = tp != null ? tp.TouchEnded : false;

                touchButton = cState.TouchButton;
                outputTouchButton = cState.OutputTouchButton;
            }
        }

        public void PopulateState(DS5State state)
        {
            unchecked
            {
                state.LX = axisdirs[(int)DS5Controls.LXNeg];
                state.LX = axisdirs[(int)DS5Controls.LXPos];
                state.LY = axisdirs[(int)DS5Controls.LYNeg];
                state.LY = axisdirs[(int)DS5Controls.LYPos];
                state.OutputLSOuter = triggers[(int)DS5Controls.LSOuter];

                state.RX = axisdirs[(int)DS5Controls.RXNeg];
                state.RX = axisdirs[(int)DS5Controls.RXPos];
                state.RY = axisdirs[(int)DS5Controls.RYNeg];
                state.RY = axisdirs[(int)DS5Controls.RYPos];
                state.OutputRSOuter = triggers[(int)DS5Controls.RSOuter];

                state.L2 = triggers[(int)DS5Controls.L2];
                state.R2 = triggers[(int)DS5Controls.R2];

                state.L1 = buttons[(int)DS5Controls.L1];
                state.L3 = buttons[(int)DS5Controls.L3];
                state.R1 = buttons[(int)DS5Controls.R1];
                state.R3 = buttons[(int)DS5Controls.R3];

                state.Cross = buttons[(int)DS5Controls.Cross];
                state.Triangle = buttons[(int)DS5Controls.Triangle];
                state.Circle = buttons[(int)DS5Controls.Circle];
                state.Square = buttons[(int)DS5Controls.Square];
                state.PS = buttons[(int)DS5Controls.PS];
                state.Options = buttons[(int)DS5Controls.Options];
                state.Share = buttons[(int)DS5Controls.Share];
                state.Mute = buttons[(int)DS5Controls.Mute];
                state.FnL = buttons[(int)DS5Controls.FnL];
                state.FnR = buttons[(int)DS5Controls.FnR];
                state.BLP = buttons[(int)DS5Controls.BLP];
                state.BRP = buttons[(int)DS5Controls.BRP];
                state.Capture = buttons[(int)DS5Controls.Capture];
                state.SideL = buttons[(int)DS5Controls.SideL];
                state.SideR = buttons[(int)DS5Controls.SideR];

                state.DpadUp = buttons[(int)DS5Controls.DpadUp];
                state.DpadRight = buttons[(int)DS5Controls.DpadRight];
                state.DpadDown = buttons[(int)DS5Controls.DpadDown];
                state.DpadLeft = buttons[(int)DS5Controls.DpadLeft];
                state.TouchButton = touchButton;
                state.OutputTouchButton = outputTouchButton;
            }
        }
    }
}
