import { useEffect, useRef, useState } from "react";

type WatchEvent = { id: number; raw: string };
type ConnectionState = "connecting" | "open" | "closed" | "error";

const MAX_EVENTS = 200;
const RECONNECT_DELAY_MS = 1_000;

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
    let reconnectTimer: number | null = null;
    let cancelled = false;

    const clearTimer = () => {
      if (reconnectTimer !== null) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    const scheduleReconnect = (nextStatus: ConnectionState) => {
      if (cancelled) {
        return;
      }
      setStatus(nextStatus);
      clearTimer();
      reconnectTimer = window.setTimeout(() => {
        reconnectTimer = null;
        connect();
      }, RECONNECT_DELAY_MS);
    };

    const connect = () => {
      if (cancelled) {
        return;
      }

      setStatus("connecting");
      const socket = new WebSocket(url);
      wsRef.current = socket;

      socket.onopen = () => setStatus("open");
      socket.onclose = () => scheduleReconnect("closed");
      socket.onerror = () => scheduleReconnect("error");
      socket.onmessage = (event) => {
        const raw = typeof event.data === "string" ? event.data : "";
        setEvents((prev) => {
          const next: WatchEvent = {
            id: prev.length === 0 ? 1 : prev[0].id + 1,
            raw,
          };
          return [next, ...prev].slice(0, MAX_EVENTS);
        });
      };
    };

    connect();

    return () => {
      cancelled = true;
      clearTimer();
      wsRef.current?.close();
      wsRef.current = null;
    };
  }, [url]);

  return (
    <main>
      <header>
        <h1>watch viewer</h1>
        <p>ws endpoint: {url}</p>
      </header>
      <p>status: {status}</p>

      <div className="layout">
        <section className="pane pane-primary">
          <h2>stream</h2>
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

        <section className="pane pane-secondary">
          <h2>latest</h2>
          {events[0] ? <pre>{events[0].raw}</pre> : <p>none yet.</p>}

          <h3>stats</h3>
          <ul>
            <li>events buffered: {events.length}</li>
            <li>max buffer: {MAX_EVENTS}</li>
            <li>reconnect delay: {RECONNECT_DELAY_MS}ms</li>
          </ul>
        </section>
      </div>
    </main>
  );
}

export default App;
