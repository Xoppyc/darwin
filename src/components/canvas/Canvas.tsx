import { onMount, onCleanup, createSignal } from "solid-js";
import {
  Application,
  Container,
  Graphics,
  TilingSprite,
  Texture,
} from "pixi.js";
import initWasm, {
  DarwinEngine,
  type InitOutput,
} from "../../engine/darwin_engine";
import wasmUrl from "../../engine/darwin_engine_bg.wasm?url";
import "./Canvas.scss";

interface DragState {
  isPanning: boolean;
  draggedNodeId: number | null;
  dragStartX: number;
  dragStartY: number;
  initialNodeX: number;
  initialNodeY: number;
  lastPointerX: number;
  lastPointerY: number;
}

export function Canvas() {
  let containerRef!: HTMLDivElement;
  let canvasRef!: HTMLCanvasElement;

  const [stats, setStats] = createSignal({
    zoom: 1.0,
    cameraX: 0,
    cameraY: 0,
    nodeCount: 0,
    edgeCount: 0,
  });

  onMount(async () => {
    let wasm: InitOutput;
    try {
      wasm = await initWasm({ module_or_path: wasmUrl });
    } catch {
      wasm = await initWasm(wasmUrl);
    }

    const engine = new DarwinEngine();

    // shit to test the shi cuh
    // these ai videos with "cuh" and mosqito with trumpet song are frying my brain
    engine.add_node(1, 100, 100, 240, 160);
    engine.add_node(2, 420, 140, 260, 210);
    engine.add_node(3, 760, 280, 230, 150);

    // Initial relationships to test dynamic port flipping and orthogonal wire routing
    engine.add_edge(1, 1, 2, 1, 3);
    engine.add_edge(2, 2, 3, 1, 3);

    const app = new Application();
    await app.init({
      canvas: canvasRef,
      resizeTo: containerRef,
      backgroundColor: 0x1e1f22,
      resolution: window.devicePixelRatio || 1,
      autoDensity: true,
      antialias: true,
    });

    // 1 draw call for dotted bg
    const gridCanvas = document.createElement("canvas");
    gridCanvas.width = 32;
    gridCanvas.height = 32;
    const gridCtx = gridCanvas.getContext("2d")!;
    gridCtx.fillStyle = "#2d2f34";
    gridCtx.fillRect(0, 0, 1.5, 1.5);
    const gridTexture = Texture.from(gridCanvas);

    const grid = new TilingSprite({
      texture: gridTexture,
      width: app.screen.width,
      height: app.screen.height,
    });

    // that's the thing that holds all world entities (nodes)
    const world = new Container();
    const edgesGraphics = new Graphics();
    const nodesContainer = new Container();
    world.addChild(edgesGraphics);
    world.addChild(nodesContainer);

    app.stage.addChild(grid);
    app.stage.addChild(world);

    const nodeGfxMap = new Map<number, Container>();

    function buildNodeCard(
      id: number,
      width: number,
      height: number,
    ): Container {
      const card = new Container();
      const body = new Graphics();

      // rounded card chassis - darcula neutral dark
      body.roundRect(0, 0, width, height, 6);
      body.fill({ color: 0x2b2d30, alpha: 0.98 });
      body.stroke({ width: 1, color: 0x393b40 });

      // table header strip
      body.roundRect(0, 0, width, 32, 6);
      body.fill({ color: 0x313438 });
      body.stroke({ width: 1, color: 0x393b40 });

      // horizontal divider between header and columns
      body.moveTo(0, 32);
      body.lineTo(width, 32);
      body.stroke({ width: 1, color: 0x393b40 });

      card.addChild(body);
      return card;
    }

    // main perfomance boost imo
    // zero-copy sync: direct typed view over Wasm linear memory
    function syncSceneFromBuffer() {
      if (!engine.is_dirty()) {
        return;
      }

      const stride = DarwinEngine.node_stride();
      const count = engine.nodes_count();
      const ptr = engine.nodes_ptr();
      const buffer = new Float32Array(wasm.memory.buffer, ptr, count * stride);

      for (let i = 0; i < count; i++) {
        const offset = i * stride;
        const id = buffer[offset + 0];
        const x = buffer[offset + 1];
        const y = buffer[offset + 2];
        const w = buffer[offset + 3];
        const h = buffer[offset + 4];

        let card = nodeGfxMap.get(id);
        if (!card) {
          card = buildNodeCard(id, w, h);
          nodeGfxMap.set(id, card);
          nodesContainer.addChild(card);
        }

        card.position.set(x, y);
      }

      // Read calculated orthogonal wires directly from Wasm edge buffer
      edgesGraphics.clear();
      const edgeCount = engine.edges_count();
      const edgeStride = DarwinEngine.edge_stride();
      const edgeBuffer = new Float32Array(
        wasm.memory.buffer,
        engine.edges_ptr(),
        edgeCount * edgeStride,
      );

      for (let i = 0; i < edgeCount; i++) {
        const offset = i * edgeStride;
        const ptCount = edgeBuffer[offset + 1];
        const tgtNormal = edgeBuffer[offset + 5];

        const pts: Array<{ x: number; y: number }> = [];
        const pBase = offset + 6;
        for (let p = 0; p < ptCount; p++) {
          pts.push({
            x: edgeBuffer[pBase + p * 2 + 0],
            y: edgeBuffer[pBase + p * 2 + 1],
          });
        }

        if (pts.length < 2) continue;

        // Draw orthogonal wire with adaptive corner fillets
        edgesGraphics.moveTo(pts[0].x, pts[0].y);
        for (let p = 1; p < pts.length - 1; p++) {
          const seg1 = Math.hypot(
            pts[p].x - pts[p - 1].x,
            pts[p].y - pts[p - 1].y,
          );
          const seg2 = Math.hypot(
            pts[p + 1].x - pts[p].x,
            pts[p + 1].y - pts[p].y,
          );
          const cornerRadius = Math.min(8, seg1 * 0.5, seg2 * 0.5);
          edgesGraphics.arcTo(
            pts[p].x,
            pts[p].y,
            pts[p + 1].x,
            pts[p + 1].y,
            cornerRadius,
          );
        }
        const last = pts[pts.length - 1];
        const prev = pts[pts.length - 2];
        edgesGraphics.lineTo(last.x, last.y);
        edgesGraphics.stroke({ width: 1.5, color: 0x5e6475, alpha: 0.9 });

        // Source anchor dot (Darcula amber)
        edgesGraphics.circle(pts[0].x, pts[0].y, 3);
        edgesGraphics.fill({ color: 0xcc7832 });

        // Crow's foot marker spreading backwards along incoming wire
        const incomingDx = last.x - prev.x;
        const backDir = incomingDx >= 0 ? -1 : 1;
        const forkLen = 10;
        const forkSpread = 6;

        // Fork prongs
        edgesGraphics.moveTo(last.x, last.y - forkSpread);
        edgesGraphics.lineTo(last.x + backDir * forkLen, last.y);
        edgesGraphics.lineTo(last.x, last.y + forkSpread);

        // Crow's foot crossbar
        edgesGraphics.moveTo(last.x + backDir * 12, last.y - 6);
        edgesGraphics.lineTo(last.x + backDir * 12, last.y + 6);
        edgesGraphics.stroke({ width: 1.5, color: 0x9876aa });
      }

      engine.clear_dirty();
      syncCamera();
    }

    function syncCamera() {
      const camX = engine.camera_x();
      const camY = engine.camera_y();
      const zoom = engine.zoom();

      world.position.set(camX, camY);
      world.scale.set(zoom, zoom);

      grid.tilePosition.set(camX, camY);
      grid.tileScale.set(zoom, zoom);

      setStats({
        zoom: Math.round(zoom * 100) / 100,
        cameraX: Math.round(camX),
        cameraY: Math.round(camY),
        nodeCount: engine.nodes_count(),
        edgeCount: engine.edges_count(),
      });
    }

    const drag: DragState = {
      isPanning: false,
      draggedNodeId: null,
      dragStartX: 0,
      dragStartY: 0,
      initialNodeX: 0,
      initialNodeY: 0,
      lastPointerX: 0,
      lastPointerY: 0,
    };

    function onPointerDown(e: PointerEvent) {
      drag.lastPointerX = e.clientX;
      drag.lastPointerY = e.clientY;

      //  I can't come up with how to make Space+Left work at the moment
      if (e.button === 1) {
        drag.isPanning = true;
        canvasRef.setPointerCapture(e.pointerId);
        return;
      }

      if (e.button === 0) {
        const worldX = engine.screen_to_world_x(e.clientX);
        const worldY = engine.screen_to_world_y(e.clientY);
        const hitId = engine.hit_test(worldX, worldY);

        if (hitId > 0) {
          console.log("[spine] hit node card:", hitId);
          drag.draggedNodeId = hitId;
          drag.dragStartX = worldX;
          drag.dragStartY = worldY;

          // we use linear memory here to ensure 0 latency and remapping or whatever
          const stride = DarwinEngine.node_stride();
          const buffer = new Float32Array(
            wasm.memory.buffer,
            engine.nodes_ptr(),
            engine.nodes_len(),
          );
          for (let i = 0; i < engine.nodes_count(); i++) {
            if (Math.abs(buffer[i * stride] - hitId) < 1e-4) {
              drag.initialNodeX = buffer[i * stride + 1];
              drag.initialNodeY = buffer[i * stride + 2];
              break;
            }
          }
          canvasRef.setPointerCapture(e.pointerId);
        } else {
          drag.isPanning = true;
          canvasRef.setPointerCapture(e.pointerId);
        }
      }
    }

    function onPointerMove(e: PointerEvent) {
      if (drag.isPanning) {
        const dx = e.clientX - drag.lastPointerX;
        const dy = e.clientY - drag.lastPointerY;
        drag.lastPointerX = e.clientX;
        drag.lastPointerY = e.clientY;
        engine.pan(dx, dy);
        syncCamera();
        return;
      }

      if (drag.draggedNodeId !== null) {
        const currentWorldX = engine.screen_to_world_x(e.clientX);
        const currentWorldY = engine.screen_to_world_y(e.clientY);
        const deltaWorldX = currentWorldX - drag.dragStartX;
        const deltaWorldY = currentWorldY - drag.dragStartY;

        engine.move_node(
          drag.draggedNodeId,
          drag.initialNodeX + deltaWorldX,
          drag.initialNodeY + deltaWorldY,
        );
        syncSceneFromBuffer();
      }
    }

    function onPointerUp(e: PointerEvent) {
      if (drag.isPanning || drag.draggedNodeId !== null) {
        drag.isPanning = false;
        drag.draggedNodeId = null;
        // I really do hope that pointer capture won't explode
        try {
          canvasRef.releasePointerCapture(e.pointerId);
        } catch {
          // pointer capture may have been released natively on blur
        }
      }
    }

    function onWheel(e: WheelEvent) {
      e.preventDefault();

      // trackpad pinch-to-zoom sets ctrlKey; otherwise wheel zooms centered on pointer
      const zoomFactor = e.deltaY < 0 ? 1.08 : 0.925;
      engine.zoom_at(e.clientX, e.clientY, zoomFactor, 0.15, 3.5);
      syncCamera();
    }

    function onResize() {
      grid.width = app.screen.width;
      grid.height = app.screen.height;
    }

    canvasRef.addEventListener("pointerdown", onPointerDown);
    canvasRef.addEventListener("pointermove", onPointerMove);
    canvasRef.addEventListener("pointerup", onPointerUp);
    canvasRef.addEventListener("pointercancel", onPointerUp);
    canvasRef.addEventListener("wheel", onWheel, { passive: false });
    window.addEventListener("resize", onResize);

    // init buffer projection
    syncSceneFromBuffer();

    onCleanup(() => {
      canvasRef.removeEventListener("pointerdown", onPointerDown);
      canvasRef.removeEventListener("pointermove", onPointerMove);
      canvasRef.removeEventListener("pointerup", onPointerUp);
      canvasRef.removeEventListener("pointercancel", onPointerUp);
      canvasRef.removeEventListener("wheel", onWheel);
      window.removeEventListener("resize", onResize);
      app.destroy(true, { children: true, texture: true });
      engine.free();
    });
  });

  return (
    <div class="darwin-canvas-root" ref={containerRef}>
      <canvas ref={canvasRef} />
      <div class="canvas-hud-overlay">
        <span>
          Zoom: <span class="hud-badge">{Math.round(stats().zoom * 100)}%</span>
        </span>
        <div class="hud-divider" />
        <span>
          Cam:{" "}
          <span class="hud-badge">
            {stats().cameraX}, {stats().cameraY}
          </span>
        </span>
        <div class="hud-divider" />
        <span>
          Nodes: <span class="hud-badge">{stats().nodeCount}</span>
        </span>
        <div class="hud-divider" />
        <span>
          Edges: <span class="hud-badge">{stats().edgeCount}</span>
        </span>
      </div>
    </div>
  );
}
