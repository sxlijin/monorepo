import { useEffect, useMemo, useRef, useState } from "react";

type WatchEvent = { id: number; raw: string; data?: EventPayload };
type EventPayload = { paths?: string[]; [key: string]: unknown };
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
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [collapsedPaths, setCollapsedPaths] = useState<Set<string>>(
    () => new Set()
  );
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
        let data: EventPayload | undefined = undefined;

        try {
          const parsed = JSON.parse(raw);
          if (parsed && typeof parsed === "object") {
            data = parsed as EventPayload;
          }
        } catch (_) {
          // keep raw message as-is
        }

        setEvents((prev) => {
          const next: WatchEvent = {
            id: prev.length === 0 ? 1 : prev[0].id + 1,
            raw,
            data,
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

  const tree = useMemo(() => {
    const pathArrays = events.flatMap((event) =>
      Array.isArray(event.data?.paths)
        ? event.data.paths.map((p) => splitPath(p))
        : []
    );
    return buildTree(pathArrays);
  }, [events]);

  const filteredEvents = useMemo(() => {
    if (!selectedPath) {
      return events;
    }

    return events.filter((event) => {
      if (!Array.isArray(event.data?.paths)) {
        return false;
      }

      return event.data!.paths!.some((path) => path.startsWith(selectedPath));
    });
  }, [events, selectedPath]);

  const timelineWindowMs = 5 * 60 * 1000;
  const timelineNow = Date.now();
  const timelineStart = timelineNow - timelineWindowMs;
  const timelineRange = Math.max(timelineWindowMs, 1);
  const timelineEvents = events
    .map((event) => {
      const timestamp = timestampToNumber(event.data?.timestamp);
      return timestamp == null ? null : { id: event.id, timestamp };
    })
    .filter(
      (entry): entry is { id: number; timestamp: number } =>
        entry !== null && entry.timestamp >= timelineStart
    );

  return (
    <main>
      <header>
        <h1>watch viewer</h1>
        <p>ws endpoint: {url}</p>
      </header>
      <p>status: {status}</p>

      <section className="timeline-horizontal">
        <div className="timeline-horizontal-header">
          <h2>last 5 minutes</h2>
          <span>
            {new Date(timelineStart).toLocaleTimeString()} –{" "}
            {new Date(timelineNow).toLocaleTimeString()}
          </span>
        </div>
        <div className="timeline-track">
          {timelineEvents.map((event) => {
            const clamped = Math.min(
              Math.max(event.timestamp, timelineStart),
              timelineNow
            );
            const pct = ((clamped - timelineStart) / timelineRange) * 100;
            return (
              <span
                key={`timeline-dot-${event.id}`}
                className="timeline-dot"
                style={{ left: `${pct}%` }}
                title={new Date(event.timestamp).toLocaleTimeString()}
              />
            );
          })}
        </div>
      </section>

      <div className="layout">
        <section className="pane pane-secondary">
          <h2>paths</h2>
          {tree.length === 0 ? (
            <p>no file paths yet.</p>
          ) : (
            <TreeView
              nodes={tree}
              selected={selectedPath}
              collapsed={collapsedPaths}
              onSelect={(path) =>
                setSelectedPath((prev) =>
                  prev === path ? null : path ?? null
                )
              }
              onToggle={(path) =>
                setCollapsedPaths((prev) => {
                  const next = new Set(prev);
                  if (next.has(path)) {
                    next.delete(path);
                  } else {
                    next.add(path);
                  }
                  return next;
                })
              }
            />
          )}
          <button
            type="button"
            className="clear"
            onClick={() => setSelectedPath(null)}
            disabled={!selectedPath}
          >
            clear filter
          </button>
        </section>

        <section className="pane pane-primary">
          <h2>stream</h2>
          {filteredEvents.length === 0 ? (
            <p>waiting for messages...</p>
          ) : (
            <ol>
              {filteredEvents.map((event) => (
                <li key={event.id}>
                  <pre>{event.raw}</pre>
                </li>
              ))}
            </ol>
          )}

          <div className="stats">
            <p>events buffered: {events.length}</p>
            <p>max buffer: {MAX_EVENTS}</p>
            <p>reconnect delay: {RECONNECT_DELAY_MS}ms</p>
            <p>
              active filter:{" "}
              {selectedPath ? <code>{selectedPath}</code> : "none"}
            </p>
          </div>
        </section>
      </div>
    </main>
  );
}

export default App;

type TreeNode = {
  name: string;
  fullPath: string;
  count: number;
  children: TreeNode[];
};

function TreeView({
  nodes,
  selected,
  collapsed,
  onSelect,
  onToggle,
}: {
  nodes: TreeNode[];
  selected: string | null;
  collapsed: Set<string>;
  onSelect: (path: string | null) => void;
  onToggle: (path: string) => void;
}) {
  if (nodes.length === 0) {
    return null;
  }

  return (
    <ul className="tree">
      {nodes.map((node) => (
        <li key={node.fullPath}>
          <div className="tree-row">
            {node.children.length > 0 ? (
              <button
                type="button"
                className="collapse-btn"
                onClick={() => onToggle(node.fullPath)}
              >
                {collapsed.has(node.fullPath) ? "+" : "-"}
              </button>
            ) : (
              <span className="collapse-placeholder" />
            )}
            <button
              type="button"
              className={
                selected === node.fullPath ? "tree-node selected" : "tree-node"
              }
              onClick={() => onSelect(node.fullPath)}
            >
              {node.name} <span className="count">({node.count})</span>
            </button>
          </div>
          {node.children.length > 0 && !collapsed.has(node.fullPath) ? (
            <TreeView
              nodes={node.children}
              selected={selected}
              collapsed={collapsed}
              onSelect={onSelect}
              onToggle={onToggle}
            />
          ) : null}
        </li>
      ))}
    </ul>
  );
}

function splitPath(path: string): string[] {
  const cleaned = path.trim().replace(/^\.\/+/, "");
  const parts = cleaned.split("/").filter((segment) => segment.length > 0);
  return parts.length > 0 ? parts : ["./"];
}

function buildTree(paths: string[][]): TreeNode[] {
  type MutableNode = {
    name: string;
    fullPath: string;
    count: number;
    children: Map<string, MutableNode>;
  };

  const root = new Map<string, MutableNode>();

  for (const segments of paths) {
    let cursor = root;
    const acc: string[] = [];

    segments.forEach((segment) => {
      acc.push(segment);
      const key = segment;
      let node = cursor.get(key);
      if (!node) {
        node = {
          name: segment,
          fullPath: acc.join("/"),
          count: 0,
          children: new Map(),
        };
        cursor.set(key, node);
      }

      node.count += 1;
      cursor = node.children;
    });
  }

  const toImmutable = (map: Map<string, MutableNode>): TreeNode[] =>
    Array.from(map.values())
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((node) => ({
        name: node.name,
        fullPath: node.fullPath,
        count: node.count,
        children: toImmutable(node.children),
      }));

  return toImmutable(root);
}

function timestampToNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }

  if (typeof value === "string") {
    const numeric = Number(value);
    if (!Number.isNaN(numeric)) {
      return numeric;
    }
    const parsed = Date.parse(value);
    if (!Number.isNaN(parsed)) {
      return parsed;
    }
  }

  return null;
}
