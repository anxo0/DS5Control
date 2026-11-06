using System.Net.Http;

namespace DS5Control
{
    /// <summary>
    /// HTTP client shared by the engine and the interface. It stays apart from
    /// <see cref="Global"/> so that assigning it does not run the Global static initializers.
    /// </summary>
    public static class SharedHttpClient
    {
        public static HttpClient Instance { get; set; }
    }
}
