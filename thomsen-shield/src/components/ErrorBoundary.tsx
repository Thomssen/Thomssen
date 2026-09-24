import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** Remounts the boundary (clearing any caught error) when this changes -
   * pass the current route so navigating away from a broken page recovers. */
  resetKey?: unknown;
}

interface State {
  error: Error | null;
}

/** Catches render-time exceptions in a page so one broken page can't blank
 * the whole app - the sidebar and title bar stay usable. */
export class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidUpdate(prevProps: Props) {
    if (this.state.error && prevProps.resetKey !== this.props.resetKey) {
      this.setState({ error: null });
    }
  }

  render() {
    if (this.state.error) {
      return (
        <div style={{ padding: 32, color: "var(--text-secondary)" }}>
          <p style={{ color: "var(--text)", fontWeight: 600, fontSize: 14, marginBottom: 6 }}>Something went wrong.</p>
          <p style={{ fontSize: 12.5, color: "var(--text-muted)" }}>{this.state.error.message || "This page failed to render."}</p>
        </div>
      );
    }
    return this.props.children;
  }
}
