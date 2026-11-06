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

using System;

namespace DS5Control
{
    interface ITouchpadBehaviour
    {
        void touchesBegan(DS5Touchpad sender, TouchpadEventArgs arg);
        void touchesMoved(DS5Touchpad sender, TouchpadEventArgs arg);
        void touchButtonUp(DS5Touchpad sender, TouchpadEventArgs arg);
        void touchButtonDown(DS5Touchpad sender, TouchpadEventArgs arg);
        void touchesEnded(DS5Touchpad sender, TouchpadEventArgs arg);
        void sixaxisMoved(DS5SixAxis sender, SixAxisEventArgs unused);
        void touchUnchanged(DS5Touchpad sender, EventArgs unused);
    }
}
