import { useEffect, useRef, useState } from "react";

type WatchEvent = { id: number; raw: string };
type ConnectionState = "connecting" | "open" | "closed" | "error";

const MAX_EVENTS = 200;

const buildWsUrl = () => {
  const protocol = window.location.protocol === "https:" ? "wss" : "ws";
  const host = window.location.hostname || "127.0.0.1";
  return `${protocol}://${host}:8080/ws`;
};

function App() {
  const [events, setEvents] = useState<WatchEvent[]>([]);
  const [status, setStatus] = useState<ConnectionState>("connecting");
  const [url] = useState(buildWsUrl);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    const ws = new WebSocket(url);
    wsRef.current = ws;

    ws.onopen = () => setStatus("open");
    ws.onclose = () => setStatus("closed");
    ws.onerror = () => setStatus("error");

    ws.onmessage = (event) => {
      const raw = typeof event.data === "string" ? event.data : "";
      setEvents((prev) => {
        const next: WatchEvent = {
          id: prev.length === 0 ? 1 : prev[0].id + 1,
          raw,
        };
        return [next, ...prev].slice(0, MAX_EVENTS);
      });
    };

    return () => {
      ws.close();
      wsRef.current = null;
    };
  }, [url]);

  return (
    <main>
      <h1>watch viewer</h1>
      <p>ws endpoint: {url}</p>
      <p>status: {status}</p>

      <section>
        {events.length === 0 ? (
          <p>waiting for messages...</p>
        ) : (
          <ol>
            {events.map((event) => (
              <li key={event.id}>
                <pre>{event.raw}</pre>
              </li>
            ))}
          </ol>
        )}
      </section>
    </main>
  );
}

export default App;
