import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { TicketGeneratedEvent, ProgressEvent, TransferCompleteEvent, ErrorEvent } from "./types";
import "./App.css";

function App() {
  const [ticket, setTicket] = useState<string>("");
  const [progress, setProgress] = useState<{ sent: number; total: number } | null>(null);
  const [status, setStatus] = useState<string>("Ready");
  const [logs, setLogs] = useState<string[]>([]);

  const addLog = (msg: string) => {
    setLogs((prev) => [...prev, `[${new Date().toLocaleTimeString()}] ${msg}`]);
  };

  // Set up event listeners
  useEffect(() => {
    const unlistenTicket = listen<TicketGeneratedEvent>("forever-file://ticket-generated", (event) => {
      console.log("Ticket generated:", event.payload);
      setTicket(event.payload.ticket);
      addLog(`✅ Ticket generated: ${event.payload.ticket.substring(0, 50)}...`);
    });

    const unlistenProgress = listen<ProgressEvent>("forever-file://progress", (event) => {
      console.log("Progress:", event.payload);
      setProgress(event.payload);
      const percent = event.payload.total > 0 
        ? Math.round((event.payload.sent / event.payload.total) * 100) 
        : 0;
      addLog(`📊 Progress: ${percent}% (${event.payload.sent}/${event.payload.total} bytes)`);
    });

    const unlistenComplete = listen<TransferCompleteEvent>("forever-file://complete", (event) => {
      console.log("Complete:", event.payload);
      setStatus(event.payload.success ? "Complete!" : "Failed");
      addLog(`${event.payload.success ? "🎉" : "❌"} ${event.payload.message}`);
    });

    const unlistenError = listen<ErrorEvent>("forever-file://error", (event) => {
      console.error("Error:", event.payload);
      setStatus("Error");
      addLog(`❌ Error: ${event.payload.message}`);
    });

    // Cleanup listeners on unmount
    return () => {
      unlistenTicket.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []);

  // Test send function
  const testSend = async () => {
    try {
      setStatus("Starting send...");
      addLog("🚀 Starting file send...");
      // For testing, we'll use a placeholder path
      // In production, use Tauri's file dialog
      await invoke("start_send", { filepath: "/tmp/test.txt" });
      addLog("📤 Send command invoked");
    } catch (err) {
      addLog(`❌ Invoke failed: ${err}`);
    }
  };

  return (
    <main className="container">
      <h1>Forever File</h1>
      <p>P2P File Transfer with Iroh</p>

      <div className="row">
        <button onClick={testSend}>🚀 Test Send</button>
      </div>

      <div className="status-box">
        <h3>Status: {status}</h3>
        {ticket && (
          <div>
            <strong>Ticket:</strong>
            <code style={{ fontSize: "0.7em", wordBreak: "break-all" }}>{ticket}</code>
          </div>
        )}
        {progress && (
          <div>
            <strong>Progress:</strong> {progress.sent} / {progress.total} bytes
          </div>
        )}
      </div>

      <div className="logs">
        <h3>Event Log:</h3>
        <div style={{ 
          maxHeight: "200px", 
          overflow: "auto", 
          background: "#1a1a1a", 
          padding: "10px",
          borderRadius: "8px",
          fontSize: "0.85em"
        }}>
          {logs.length === 0 ? (
            <p style={{ color: "#666" }}>No events yet...</p>
          ) : (
            logs.map((log, i) => <div key={i}>{log}</div>)
          )}
        </div>
      </div>
    </main>
  );
}

export default App;
