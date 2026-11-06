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
using System.Collections.Generic;
using System.Threading;

namespace DS5Control
{
    public class ControllerSlotManager
    {
        private ReaderWriterLockSlim collectionLocker = new ReaderWriterLockSlim();
        public ReaderWriterLockSlim CollectionLocker { get => collectionLocker; }

        private List<DS5Device> controllerColl;
        public List<DS5Device> ControllerColl { get => controllerColl; set => controllerColl = value; }
        
        private Dictionary<int, DS5Device> controllerDict;
        private Dictionary<DS5Device, int> reverseControllerDict;
        public Dictionary<int, DS5Device> ControllerDict { get => controllerDict; }
        public Dictionary<DS5Device, int> ReverseControllerDict { get => reverseControllerDict; }

        public ControllerSlotManager()
        {
            controllerColl = new List<DS5Device>();
            controllerDict = new Dictionary<int, DS5Device>();
            reverseControllerDict = new Dictionary<DS5Device, int>();
        }

        public void AddController(DS5Device device, int slotIdx)
        {
            using (WriteLocker locker = new WriteLocker(collectionLocker))
            {
                controllerColl.Add(device);
                controllerDict.Add(slotIdx, device);
                reverseControllerDict.Add(device, slotIdx);
            }
        }

        public void RemoveController(DS5Device device, int slotIdx)
        {
            using (WriteLocker locker = new WriteLocker(collectionLocker))
            {
                controllerColl.Remove(device);
                controllerDict.Remove(slotIdx);
                reverseControllerDict.Remove(device);
            }
        }

        public void ClearControllerList()
        {
            using (WriteLocker locker = new WriteLocker(collectionLocker))
            {
                controllerColl.Clear();
                controllerDict.Clear();
                reverseControllerDict.Clear();
            }
        }
    }
}
