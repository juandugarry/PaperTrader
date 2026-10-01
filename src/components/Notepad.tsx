import { useState, useEffect, useCallback, type FormEvent } from "react";
import {
  saveNote,
  deleteNote,
  getSnapshot,
  type Snapshot,
  type Note,
} from "../api";
export default function Notepad({
  snapshot,
  onChanged,
  onDirtyChange,
}: {
  snapshot: Snapshot;
  onChanged: (s: Snapshot) => void;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [selected, setSelected] = useState<number | null>(null),
    [creating, setCreating] = useState(false),
    [query, setQuery] = useState("");
  const [draftKey, setDraftKey] = useState(0);
  const [dirty, setDirty] = useState(false),
    [pending, setPending] = useState<(() => void) | null>(null);
  const reportDirty = useCallback(
    (value: boolean) => {
      setDirty(value);
      onDirtyChange?.(value);
    },
    [onDirtyChange],
  );
  function request(action: () => void) {
    if (dirty) setPending(() => action);
    else action();
  }
  const notes = snapshot.notes.filter((n) =>
    `${n.title} ${n.body}`.toLowerCase().includes(query.toLowerCase()),
  );
  const note =
    (dirty ? snapshot.notes : notes).find((n) => n.id === selected) ??
    (dirty ? snapshot.notes[0] : notes[0]);
  return (
    <>
      {pending && (
        <div className="unsaved-confirm">
          <p>
            There are unsaved note changes. Save first, or discard them to
            continue.
          </p>
          <button
            className="danger-outline"
            onClick={() => {
              const action = pending;
              setPending(null);
              reportDirty(false);
              action();
            }}
          >
            Discard changes and continue
          </button>
          <button className="secondary" onClick={() => setPending(null)}>
            Keep editing
          </button>
        </div>
      )}
      <div className="notepad-heading">
        <div>
          <h2>Your notepad</h2>
          <p>
            Ideas, observations and questions. Saved locally, separate from
            trade records.
          </p>
        </div>
        <button
          onClick={() =>
            request(() => {
              setCreating(true);
              setDraftKey((key) => key + 1);
            })
          }
        >
          New note
        </button>
      </div>
      <label>
        Search notes
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Title or text"
        />
      </label>
      <div className="notepad-layout">
        <div className="note-list">
          {notes.map((n) => (
            <button
              className={
                !creating && note?.id === n.id
                  ? "note-card selected"
                  : "note-card"
              }
              key={n.id}
              onClick={() => {
                if (!creating && note?.id === n.id) return;
                request(() => {
                  setSelected(n.id);
                  setCreating(false);
                });
              }}
            >
              <strong>{n.title}</strong>
              <span>{n.body.slice(0, 100) || "Empty note"}</span>
              <small>{new Date(n.updatedAt).toLocaleString("en-AU")}</small>
            </button>
          ))}
          {!notes.length && (
            <p className="muted">
              {snapshot.notes.length
                ? "No matching notes."
                : "No notes yet. Create one for your next idea."}
            </p>
          )}
        </div>
        {creating ? (
          <NoteEditor
            key={`new-${draftKey}`}
            onDirtyChange={reportDirty}
            onSaved={(s) => {
              onChanged(s);
              setSelected(
                s.notes.find(
                  (n) => !snapshot.notes.some((old) => old.id === n.id),
                )?.id ?? null,
              );
              setCreating(false);
              setQuery("");
            }}
            onCancel={() => setCreating(false)}
          />
        ) : note ? (
          <NoteEditor
            key={`${note.id}-${note.version}`}
            onDirtyChange={reportDirty}
            note={note}
            onSaved={onChanged}
            onDeleted={(s) => {
              onChanged(s);
              setSelected(null);
            }}
            onReload={onChanged}
          />
        ) : (
          <section className="panel empty">
            <h3>Make space for your thinking</h3>
            <p>Keep research notes, watchlists or learning questions here.</p>
          </section>
        )}
      </div>
    </>
  );
}
function NoteEditor({
  note,
  onSaved,
  onDeleted,
  onCancel,
  onReload,
  onDirtyChange,
}: {
  note?: Note;
  onSaved: (s: Snapshot) => void;
  onDeleted?: (s: Snapshot) => void;
  onCancel?: () => void;
  onReload?: (s: Snapshot) => void;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [title, setTitle] = useState(note?.title ?? ""),
    [body, setBody] = useState(note?.body ?? ""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [saved, setSaved] = useState(false),
    [confirmDelete, setConfirmDelete] = useState(false),
    [conflict, setConflict] = useState(false);
  const dirty = note
    ? title !== note.title || body !== note.body
    : !!title || !!body;
  useEffect(() => {
    onDirtyChange?.(dirty);
  }, [dirty, onDirtyChange]);
  useEffect(() => () => onDirtyChange?.(false), [onDirtyChange]);
  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    setError("");
    try {
      onSaved(
        await saveNote({
          id: note?.id ?? null,
          expectedVersion: note?.version ?? null,
          title: title.trim(),
          body,
        }),
      );
      setSaved(true);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      if (msg.includes("another window")) setConflict(true);
    } finally {
      setBusy(false);
    }
  }
  async function remove() {
    if (!note) return;
    setBusy(true);
    setError("");
    try {
      onDeleted?.(
        await deleteNote({ id: note.id, expectedVersion: note.version }),
      );
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      if (msg.includes("another window")) setConflict(true);
    } finally {
      setBusy(false);
    }
  }
  async function reload() {
    setBusy(true);
    setError("");
    try {
      const s = await getSnapshot();
      if (!s) throw new Error("Profile unavailable.");
      onReload?.(s);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="panel note-editor">
      <form onSubmit={submit}>
        <fieldset disabled={busy}>
          <label>
            Note title
            <input
              required
              maxLength={120}
              value={title}
              onChange={(e) => {
                setTitle(e.target.value);
                setSaved(false);
              }}
              placeholder="Give your idea a name"
            />
          </label>
          <label>
            Note text
            <textarea
              rows={16}
              maxLength={50000}
              value={body}
              onChange={(e) => {
                setBody(e.target.value);
                setSaved(false);
              }}
              placeholder="Write freely…"
            />
          </label>
          <div className="review-actions">
            <button disabled={busy || (!dirty && !!note)} type="submit">
              {busy ? "Saving…" : "Save note"}
            </button>
            {onCancel && (
              <button type="button" className="secondary" onClick={onCancel}>
                Cancel
              </button>
            )}
            {note && (
              <button
                type="button"
                className="danger-outline"
                onClick={() => setConfirmDelete(true)}
              >
                Delete note…
              </button>
            )}
          </div>
          <p className="muted">
            {dirty
              ? "Unsaved changes. Save before switching notes or leaving this page."
              : note
                ? `Saved locally · ${new Date(note.updatedAt).toLocaleString("en-AU")}`
                : "Save to keep this note on your device."}
          </p>
        </fieldset>
      </form>
      {confirmDelete && (
        <div className="delete-confirm">
          <p>Delete “{note?.title}”? This cannot be undone.</p>
          <div className="review-actions">
            <button
              disabled={busy}
              type="button"
              className="danger"
              onClick={() => void remove()}
            >
              Delete this note
            </button>
            <button
              disabled={busy}
              className="secondary"
              onClick={() => setConfirmDelete(false)}
            >
              Keep note
            </button>
          </div>
        </div>
      )}
      {saved && (
        <p role="status" className="success">
          Note saved.
        </p>
      )}
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
      {conflict && (
        <button
          disabled={busy}
          className="secondary"
          onClick={() => void reload()}
        >
          Reload latest notes
        </button>
      )}
    </section>
  );
}
