import { Canvas } from "./components/canvas/Canvas";
import "./App.scss";

export default function App() {
  return (
    <div class="darwin-app-container">
      <header class="darwin-toolbar">
        <div class="brand">
          <span>Darwin</span>
        </div>
        <div class="meta">
          <span>Test test test test test</span>
        </div>
      </header>
      <main class="darwin-main-viewport">
        <Canvas />
      </main>
    </div>
  );
}
