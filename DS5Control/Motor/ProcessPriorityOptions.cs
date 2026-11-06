using System.Collections.Generic;
using System.Diagnostics;

namespace DS5Control
{
    /// <summary>
    /// Process priority classes selectable in the settings. The setting stores an index into
    /// this list, so its order is part of the saved configuration.
    /// </summary>
    public static class ProcessPriorityOptions
    {
        public static readonly List<ProcessPriorityClass> Classes =
        [
            ProcessPriorityClass.Normal, ProcessPriorityClass.AboveNormal,
            ProcessPriorityClass.High, ProcessPriorityClass.RealTime
        ];
    }
}
