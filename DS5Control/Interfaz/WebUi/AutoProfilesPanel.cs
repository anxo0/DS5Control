using System.IO;
using System.Linq;
using DS5Control;

namespace DS5ControlApp.Interfaz.WebUi
{
    /// <summary>The games that change the profile on their own when they open.</summary>
    internal sealed class AutoProfilesPanel
    {
        private readonly AutoProfileHolder holder;

        public AutoProfilesPanel(AutoProfileHolder holder)
        {
            this.holder = holder;
        }

        private static string File => Path.Combine(Global.appdatapath, "Auto Profiles.xml");

        public object[] Describe()
        {
            lock (holder.CollectionLock)
            {
                return holder.AutoProfileColl.Select(entry => (object)new
                {
                    ruta = entry.Path,
                    nombre = Path.GetFileNameWithoutExtension(entry.Path),
                    perfil = entry.ProfileNames[0] == AutoProfileEntity.NONE_STRING ? string.Empty : entry.ProfileNames[0],
                }).ToArray();
            }
        }

        public void Add(string path, string profile)
        {
            lock (holder.CollectionLock)
            {
                if (holder.AutoProfileColl.Any(entry => entry.Path == path)) return;

                var entry = new AutoProfileEntity(path, string.Empty);
                Assign(entry, profile);
                holder.AutoProfileColl.Add(entry);
            }
            holder.Save(File);
        }

        public void SetProfile(string path, string profile)
        {
            lock (holder.CollectionLock)
            {
                AutoProfileEntity entry = holder.AutoProfileColl.FirstOrDefault(item => item.Path == path);
                if (entry == null) return;
                Assign(entry, profile);
            }
            holder.Save(File);
        }

        public void Remove(string path)
        {
            lock (holder.CollectionLock)
            {
                AutoProfileEntity entry = holder.AutoProfileColl.FirstOrDefault(item => item.Path == path);
                if (entry == null) return;
                holder.AutoProfileColl.Remove(entry);
            }
            holder.Save(File);
        }

        private static void Assign(AutoProfileEntity entry, string profile)
        {
            for (int i = 0; i < entry.ProfileNames.Length; i++)
            {
                entry.ProfileNames[i] = profile;
            }
        }
    }
}
