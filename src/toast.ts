// Desktop top-level toast entry point.
//
// Loads inside an independent, transparent, borderless, always-on-top
// WebviewWindow that the Rust `show_toast` command spawns. The URL
// query string carries the payload (title / body / kind / duration /
// actions), which we read here and paint into the static HTML template
// from `toast.html`. After `duration` ms we ask Rust to dismiss the
// toast (which closes this window); the user can also click the close
// button to dismiss earlier, click anywhere on the card to focus the
// main window, or click any action button to focus the main window
// and fire an `actions:run` invoke back to the main process so the
// Rust side (or main app) can perform the action.

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

// ── Query payload ─────────────────────────────────────────────────

interface ToastAction {
  /** Stable identifier — sent back via the `actions:run` invoke. */
  id: string;
  /** Display label, e.g. "打开目录" / "Open". */
  label: string;
  /** Optional Remix Icon class (e.g. "ri-folder-open-line"). */
  icon?: string;
}

function readQuery(): {
  title: string;
  body: string;
  kind: string;
  durationMs: number;
  label: string;
  actions: ToastAction[];
} {
  const p = new URLSearchParams(window.location.search);
  // Actions are encoded as a JSON array under `actions`. The Rust
  // side percent-encodes this so it survives the query-string round
  // trip. Falling back to `[]` keeps the toast rendering even when
  // the payload omits the field.
  let actions: ToastAction[] = [];
  const raw = p.get("actions");
  if (raw) {
    try {
      const parsed = JSON.parse(decodeURIComponent(raw));
      if (Array.isArray(parsed)) actions = parsed as ToastAction[];
    } catch {
      // Malformed JSON — ignore so the rest of the toast still paints.
    }
  }
  return {
    title: p.get("title") ?? "2-Pyramid",
    body: p.get("body") ?? "",
    kind: (p.get("kind") ?? "info").toLowerCase(),
    durationMs: Math.max(1000, parseInt(p.get("duration") ?? "8000", 10) || 8000),
    label: p.get("label") ?? "",
    actions,
  };
}

// ── Rendering ─────────────────────────────────────────────────────

const ICON_CLASS: Record<string, string> = {
  info: "ri-information-line",
  success: "ri-checkbox-circle-line",
  warning: "ri-alert-line",
  error: "ri-close-circle-line",
};

function paint(payload: ReturnType<typeof readQuery>): void {
  const card = document.getElementById("toast")!;
  const icon = document.getElementById("toast-icon")!;
  const iconI = document.getElementById("toast-icon-i")!;
  const title = document.getElementById("toast-title")!;
  const text = document.getElementById("toast-text")!;
  const progress = document.getElementById("toast-progress")!;
  const actionsEl = document.getElementById("toast-actions")!;

  // Kind → icon class + colour. Always force one of the four so a
  // bogus value still falls back gracefully.
  const kind = payload.kind in ICON_CLASS ? payload.kind : "info";
  icon.classList.remove("info", "success", "warning", "error");
  icon.classList.add(kind);
  iconI.className = ICON_CLASS[kind];
  title.textContent = payload.title;
  text.textContent = payload.body;
  progress.style.animationDuration = `${payload.durationMs}ms`;

  // Card is always clickable for "focus main window". The visual
  // `.clickable` class just adds the cursor + hover affordance.
  card.classList.add("clickable");

  if (payload.actions.length > 0) {
    card.classList.add("has-actions");
    payload.actions.forEach((a) => {
      const btn = document.createElement("button");
      btn.className = "toast-action-btn";
      btn.type = "button";
      if (a.icon) {
        const i = document.createElement("i");
        i.className = a.icon;
        btn.appendChild(i);
      }
      btn.appendChild(document.createTextNode(a.label));
      btn.addEventListener("click", (ev) => {
        // Action clicks shouldn't bubble up to the card focus handler.
        ev.stopPropagation();
        runAction(a.id);
      });
      actionsEl.appendChild(btn);
    });
  }
}

// ── Lifecycle ─────────────────────────────────────────────────────

async function focusMainWindow(): Promise<void> {
  // The toast window is always a top-level WebviewWindow, so the
  // "main" label refers to the app's primary window. Showing + setting
  // focus brings it back from tray / minimized state, which is the
  // entire point of click-to-focus.
  try {
    await invoke("focus_main_window");
  } catch (e) {
    console.warn("toast: focus_main_window failed:", e);
  }
}

async function runAction(actionId: string): Promise<void> {
  // Two effects happen at once: close the toast (so it doesn't keep
  // floating on top while the action runs) and ask Rust to fire the
  // action callback. The callback is registered up-front when the
  // main app emits a `notify(actions: [...])`; we route by id.
  dismissToast();
  try {
    await invoke("run_toast_action", { actionId });
  } catch (e) {
    console.warn("toast: run_toast_action failed:", e);
  }
}

async function dismissToast(): Promise<void> {
  const card = document.getElementById("toast")!;
  card.classList.add("leaving");
  await new Promise((r) => setTimeout(r, 240));
  // Close this very window. `getCurrentWindow().close()` can fail
  // silently in some IPC states; fall back to the Rust `dismiss_toast`
  // command (which closes by label) so the toast never lingers as a
  // white ghost window after its content has faded out.
  const label = readQuery().label;
  try {
    await getCurrentWindow().close();
  } catch {
    /* ignore: window may already be gone */
  }
  if (label) {
    // Give the close above a beat; if the window is still alive
    // (close failed), ask Rust to close it by label.
    setTimeout(async () => {
      try {
        await invoke("dismiss_toast", { label });
      } catch {
        /* ignore */
      }
    }, 300);
  }
}

function wireCardClick(): void {
  document.getElementById("toast")?.addEventListener("click", () => {
    focusMainWindow();
    dismissToast();
  });
}

function wireCloseButton(): void {
  document.getElementById("toast-close")?.addEventListener("click", (e) => {
    e.stopPropagation();
    dismissToast();
  });
}

(async () => {
  const payload = readQuery();
  paint(payload);
  wireCardClick();
  wireCloseButton();

  // Auto-dismiss after duration. 280 ms headroom for the leave
  // animation (260 ms) so the card finishes sliding out before the
  // window vanishes underneath.
  setTimeout(dismissToast, payload.durationMs - 280);
})();