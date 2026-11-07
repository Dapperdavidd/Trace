/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * Three.js engines ported from design/trace-redesign-v11.html:
 *  - Field: a ground of blocks. The lens finds one, the block opens and shows the trade inside.
 *  - TraceChart: the token 3D view (price ribbon, volume, a lane per wallet that bought).
 * Plus the static SVG fallbacks used when WebGL is unavailable.
 * The code stays close to the design source on purpose so the motion matches it exactly.
 */
import * as T from "three";
import { D, SOLP } from "./example";
import { clamp, ease, hm, pf, rng } from "./viz";

export interface Engine {
  dispose: () => void;
}

export function has3D(): boolean {
  try {
    const c = document.createElement("canvas");
    return !!(c.getContext("webgl") || c.getContext("experimental-webgl"));
  } catch {
    return false;
  }
}
function reduceMotion(): boolean {
  return typeof window !== "undefined" && !!window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const FGc = 0xedf2ef;
const Gc = 0x4ae38f;
const BGc = 0x0a0b0b;

function geo(arr: number[]) {
  const g = new T.BufferGeometry();
  g.setAttribute("position", new T.Float32BufferAttribute(arr, 3));
  return g;
}
function mkL(arr: number[], color: number, op: number, o: any = {}) {
  const m: any = o.dash
    ? new T.LineDashedMaterial({ color, transparent: true, opacity: op, dashSize: o.dash, gapSize: o.gap || o.dash })
    : new T.LineBasicMaterial({ color, transparent: true, opacity: op });
  if (o.top) {
    m.depthTest = false;
    m.fog = false;
  }
  const L: any = new (o.strip ? T.Line : T.LineSegments)(geo(arr), m);
  if (o.dash) L.computeLineDistances();
  if (o.top) L.renderOrder = 10;
  return L;
}

function stage(host: HTMLElement, o: { fov: number; fogNear: number; fogFar: number }) {
  const REDUCE = reduceMotion();
  const s: any = { host, W: 1, H: 1, yaw: 0, pitch: 0, dragging: false, moved: 0, pointer: null, userUntil: 0, vis: true, raf: 0, t: 0, last: 0, ls: [] };
  const cv = document.createElement("canvas");
  cv.className = "gl";
  host.appendChild(cv);
  const lb = document.createElement("div");
  lb.className = "lbls";
  host.appendChild(lb);
  s.lb = lb;
  const R = new T.WebGLRenderer({ canvas: cv, antialias: true, powerPreference: "low-power" });
  R.setClearColor(BGc, 1);
  R.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
  s.R = R;
  s.scene = new T.Scene();
  s.scene.fog = new T.Fog(BGc, o.fogNear, o.fogFar);
  s.cam = new T.PerspectiveCamera(o.fov, 1, 0.1, 500);
  function size() {
    const b = host.getBoundingClientRect();
    if (!b.width || !b.height) return;
    s.W = b.width;
    s.H = b.height;
    R.setSize(s.W, s.H, false);
    s.cam.aspect = s.W / s.H;
    s.cam.updateProjectionMatrix();
  }
  size();
  const ro = typeof ResizeObserver !== "undefined" ? new ResizeObserver(size) : null;
  if (ro) ro.observe(host);
  function loop(now: number) {
    s.raf = 0;
    if (!s.vis || !s.update) return;
    const dt = Math.min(0.05, (now - (s.last || now)) / 1000);
    s.last = now;
    s.t += dt;
    s.update(dt, now);
    R.render(s.scene, s.cam);
    s.raf = requestAnimationFrame(loop);
  }
  const io =
    typeof IntersectionObserver !== "undefined"
      ? new IntersectionObserver((es) => {
          s.vis = !!es[0]?.isIntersecting;
          if (s.vis && !s.raf && s.update && !REDUCE) {
            s.last = 0;
            s.raf = requestAnimationFrame(loop);
          }
        })
      : null;
  if (io) io.observe(host);
  function on(t: string, f: any) {
    host.addEventListener(t, f);
    s.ls.push([t, f]);
  }
  on("pointerdown", (e: PointerEvent) => {
    s.dragging = true;
    s.moved = 0;
    s.px = e.clientX;
    s.py = e.clientY;
    s.pt = e.pointerType;
  });
  on("pointermove", (e: PointerEvent) => {
    const b = host.getBoundingClientRect();
    s.pointer = { x: e.clientX - b.left, y: e.clientY - b.top };
    if (e.pointerType === "mouse") s.userUntil = performance.now() + 2600;
    if (s.dragging) {
      const dx = e.clientX - s.px;
      const dy = e.clientY - s.py;
      s.px = e.clientX;
      s.py = e.clientY;
      s.moved += Math.abs(dx) + Math.abs(dy);
      if (Math.abs(dx) > Math.abs(dy) * 0.6 || e.pointerType === "mouse") {
        s.yaw -= dx * 0.006;
        s.pitch = clamp(s.pitch + dy * 0.004, -0.45, 0.55);
      }
      if (REDUCE && s.update) {
        s.update(0, performance.now());
        R.render(s.scene, s.cam);
      }
    }
  });
  function up(e: PointerEvent) {
    const was = s.dragging;
    s.dragging = false;
    if (was && s.moved < 6 && s.onClick) {
      const b = host.getBoundingClientRect();
      s.onClick(e.clientX - b.left, e.clientY - b.top);
    }
  }
  on("pointerup", up);
  on("pointercancel", () => {
    s.dragging = false;
  });
  on("pointerleave", () => {
    s.pointer = null;
    s.dragging = false;
  });
  s.start = (u: any) => {
    s.update = u;
    if (REDUCE) {
      u(0, performance.now());
      R.render(s.scene, s.cam);
      return;
    }
    s.raf = requestAnimationFrame(loop);
  };
  s.label = (html: string, c?: string) => {
    const d = document.createElement("div");
    d.className = "lb " + (c || "");
    d.innerHTML = "<span>" + html + "</span>";
    lb.appendChild(d);
    return d;
  };
  const v = new T.Vector3();
  s.proj = (p: any) => {
    v.copy(p).project(s.cam);
    return {
      x: (v.x * 0.5 + 0.5) * s.W,
      y: (-v.y * 0.5 + 0.5) * s.H,
      ok: v.z < 1 && v.z > -1 && v.x > -1.2 && v.x < 1.2 && v.y > -1.2 && v.y < 1.2,
    };
  };
  s.place = (el: any, p: any, show: boolean) => {
    const q = s.proj(p);
    if (!show || !q.ok) {
      if (el._o !== 0) {
        el.style.opacity = 0;
        el._o = 0;
      }
      return q;
    }
    if (el._o !== 1) {
      el.style.opacity = 1;
      el._o = 1;
    }
    el.style.transform = "translate(" + q.x.toFixed(1) + "px," + q.y.toFixed(1) + "px)";
    return q;
  };
  s.dispose = () => {
    cancelAnimationFrame(s.raf);
    s.raf = 0;
    s.update = null;
    if (ro) ro.disconnect();
    if (io) io.disconnect();
    s.ls.forEach((l: any) => host.removeEventListener(l[0], l[1]));
    s.scene.traverse((x: any) => {
      if (x.geometry) x.geometry.dispose();
      if (x.material) x.material.dispose();
    });
    R.dispose();
    try {
      R.forceContextLoss();
    } catch {
      /* ignore */
    }
    host.innerHTML = "";
  };
  s.REDUCE = REDUCE;
  return s;
}

function sph(r: number, color: number, op?: number) {
  return new T.Mesh(new T.SphereGeometry(r, 20, 14), new T.MeshBasicMaterial({ color, transparent: true, opacity: op === undefined ? 1 : op }));
}
function topMesh(g: any, color: number, op: number) {
  const m = new T.Mesh(g, new T.MeshBasicMaterial({ color, transparent: true, opacity: op, depthTest: false, fog: false, side: T.DoubleSide }));
  m.renderOrder = 10;
  return m;
}
function arcCurve(a: any, b: any, lift: number) {
  const m = a.clone().add(b).multiplyScalar(0.5);
  m.y += lift;
  return new T.QuadraticBezierCurve3(a, m, b);
}
function dotFloor(x0: number, x1: number, z0: number, z1: number, step: number) {
  const dp: number[] = [];
  for (let i = x0; i <= x1; i += step) for (let j = z0; j <= z1; j += step) dp.push(i, 0, j);
  return new T.Points(geo(dp), new T.PointsMaterial({ color: FGc, size: 2, sizeAttenuation: false, transparent: true, opacity: 0.2 }));
}

export interface Txn {
  k?: string;
  hash: string;
  blk?: string;
  time?: string;
  from?: string;
  to?: string;
  rows: string[][];
  bal?: string[][];
  tag: [string, string];
}

export const TXNS: Record<string, Txn> = {
  A: { hash: "0x8f2a…41c", blk: "18,240,113", time: "09:24:13", from: "0xSun", to: "Swap", rows: [["Sent", "126.5 SOL", "$18,001"], ["Received", "3.05M NEURA", "$18,000"], ["Network fee", "0.00021 SOL", "$0.03"]], bal: [["SOL", "2,412.8", "2,286.3"], ["NEURA", "0", "3.05M"]], tag: ["Proven wallet · right 78% of the time", "g"] },
};

export function txnHtml(t: Txn, compact?: boolean): string {
  let h = '<div class="txn' + (compact ? " c" : "") + '"><div class="tx-h"><span class="tx-k">' + (t.k || "TXN") + "</span><b>" + t.hash + "</b>" + (t.blk ? '<span class="tx-b">' + (/^\d/.test(t.blk) ? "Block #" : "") + t.blk + "</span>" : "") + "</div>";
  if (t.from && !(compact && !t.to))
    h += '<div class="tx-ft rv"><span class="tx-w">' + t.from + "</span>" + (t.to ? '<span>→</span><span class="tx-w">' + t.to + "</span>" : "") + (t.time && !compact ? '<span class="tx-t">' + t.time + "</span>" : "") + "</div>";
  (compact ? t.rows.slice(0, 2) : t.rows).forEach((r) => {
    h += '<div class="tx-r rv"><span>' + r[0] + "</span><b>" + r[1] + "</b><em>" + (r[2] || "") + "</em></div>";
  });
  if (t.bal && !compact)
    h += '<div class="tx-bal rv"><span>' + t.from + " balance</span>" + t.bal.map((b) => "<div><span>" + b[0] + "</span><b>" + b[1] + " → <i>" + b[2] + "</i></b></div>").join("") + "</div>";
  return h + '<span class="tag ' + t.tag[1] + ' rv">' + t.tag[0] + "</span></div>";
}

function addrR(r: () => number) {
  const H = "0123456789abcdef";
  let s = "";
  for (let i = 0; i < 4; i++) s += H[Math.floor(r() * 16)];
  return s + "…" + H[Math.floor(r() * 16)] + H[Math.floor(r() * 16)] + H[Math.floor(r() * 16)];
}
function genTxn(b: any): Txn {
  const r = rng(b.num);
  const coin = D.order[Math.floor(r() * 6)];
  const sol = (2 + r() * 60).toFixed(1);
  const usd = Math.round(Number(sol) * SOLP);
  const amt = usd / D.coins[coin as string]!.p;
  const proven = b.num % 3 === 0;
  return {
    hash: "0x" + addrR(r).slice(0, 9),
    blk: b.num.toLocaleString("en-US"),
    time: hm(r() * 190),
    from: "Wallet " + addrR(r).slice(0, 5) + "…",
    to: "Swap",
    rows: [["Sent", sol + " SOL", "$" + usd.toLocaleString()], ["Received", (amt >= 1e6 ? (amt / 1e6).toFixed(2) + "M" : Math.round(amt).toLocaleString()) + " " + coin, ""]],
    tag: proven ? ["Proven wallet · right 74% of the time", "g"] : ["Ordinary wallet", ""],
  };
}

export interface FieldConfig {
  seed?: number;
  cols: number;
  rows: number;
  fov?: number;
  lensR?: number;
  fogNear?: number;
  fogFar?: number;
  distMul?: number;
  compact?: boolean;
  cardLift?: number;
  cardTop?: number | (() => number);
  wanderDur?: number;
  aspectFix?: boolean;
  chain?: number;
  autoOpen?: number;
  keys: Record<string, [number, number]>;
  hist?: [number, number][];
  wash?: [number, number][];
  scenes: any[];
  at: () => number;
  smooth?: number;
  wanderPick?: (b: any) => boolean;
  onPeek?: (b: any) => void;
  onClickBlock?: (b: any, auto?: boolean) => void;
}

export function Field(host: HTMLElement, cfg: FieldConfig): Engine {
  const S = stage(host, { fov: cfg.fov || 38, fogNear: cfg.fogNear || 40, fogFar: cfg.fogFar || 110 });
  const sc = S.scene;
  const r = rng(cfg.seed || 5);
  const TS = 80;
  const RS = 8;
  const inst = S.REDUCE;
  const cols = cfg.cols;
  const rows = cfg.rows;
  const SX = 2.4;
  const SZ = 2.7;
  const X0 = (-(cols - 1) * SX) / 2;
  const Z0 = (-(rows - 1) * SZ) / 2;
  let i: number;
  let j: number;
  const boxG = new T.BoxGeometry(1.3, 1, 1.3);
  const edgeG = new T.EdgesGeometry(boxG);
  const fM = new T.MeshBasicMaterial({ color: 0x131816 });
  const fG = new T.MeshBasicMaterial({ color: 0x163323 });
  const eDim = new T.LineBasicMaterial({ color: FGc, transparent: true, opacity: 0.24 });
  const eHi = new T.LineBasicMaterial({ color: FGc, transparent: true, opacity: 0.85 });
  const eG = new T.LineBasicMaterial({ color: Gc });
  const blocks: any[] = [];
  const grid: Record<string, any> = {};
  for (i = 0; i < rows; i++)
    for (j = 0; j < cols; j++) {
      const h = 0.7 + r() * 0.6;
      const g = new T.Group();
      const m = new T.Mesh(boxG, fM);
      const e = new T.LineSegments(edgeG, eDim);
      g.add(m);
      g.add(e);
      const x = X0 + j * SX;
      const z = Z0 + i * SZ;
      g.scale.set(1, h, 1);
      g.position.set(x, h / 2, z);
      sc.add(g);
      const b = { r: i, c: j, x, z, h, g, m, e, lift: 0, pop: -1, num: 18240000 + (i * cols + j) * 7 + 101 };
      blocks.push(b);
      grid[i + "," + j] = b;
    }
  const rowA: any[] = [];
  for (i = 0; i < rows; i++) rowA.push(-1);
  let chainT = 0;
  let topNum = 18240000 + rows * cols * 7 + 101;
  let wPicks = 0;
  const B = (rc: [number, number]) => grid[rc[0] + "," + rc[1]];
  const keyB: Record<string, any> = {};
  Object.keys(cfg.keys).forEach((k) => {
    const b = B(cfg.keys[k] as [number, number]);
    keyB[k] = b;
    const t = TXNS[k];
    if (t && /^\d/.test(t.blk || "")) b.num = parseInt((t.blk || "").replace(/,/g, ""));
  });
  const lp: number[] = [];
  for (i = 0; i < rows; i++)
    for (j = 0; j < cols - 1; j++) {
      const z2 = Z0 + i * SZ;
      lp.push(X0 + j * SX + 0.65, 0.25, z2, X0 + (j + 1) * SX - 0.65, 0.25, z2);
    }
  sc.add(mkL(lp, FGc, 0.16));
  const pks: any[] = [];
  for (i = 0; i < Math.round(rows * 1.3); i++) {
    const pk = sph(0.11, Gc, 0.9);
    pk.userData = { row: Math.floor(r() * rows), x: X0 + r() * (cols - 1) * SX, v: 1.6 + r() * 1.6 };
    sc.add(pk);
    pks.push(pk);
  }
  const topOf = (b: any) => new T.Vector3(b.x, b.g.position.y + b.g.scale.y / 2, b.z);
  const hist = (cfg.hist || []).map(B);
  const hArcs: any[] = [];
  for (i = 1; i < hist.length; i++) {
    const a = hist[i - 1];
    const b2 = hist[i];
    const cv = arcCurve(new T.Vector3(a.x, a.h + 0.05, a.z), new T.Vector3(b2.x, b2.h + 0.05, b2.z), 2.4);
    const tb = new T.Mesh(new T.TubeGeometry(cv, TS, 0.06, RS, false), new T.MeshBasicMaterial({ color: Gc, transparent: true }));
    tb.geometry.setDrawRange(0, 0);
    sc.add(tb);
    hArcs.push(tb);
  }
  const wash = (cfg.wash || []).map(B);
  const wCurves: any[] = [];
  const wLines: any[] = [];
  let wPk: any = null;
  let wLab: any = null;
  const wCenter = new T.Vector3();
  if (wash.length) {
    wash.forEach((b) => {
      wCenter.x += b.x / wash.length;
      wCenter.z += b.z / wash.length;
    });
    for (i = 0; i < wash.length; i++) {
      const a2 = wash[i];
      const b3 = wash[(i + 1) % wash.length];
      const cv2 = arcCurve(new T.Vector3(a2.x, a2.h + 0.05, a2.z), new T.Vector3(b3.x, b3.h + 0.05, b3.z), 1.4);
      wCurves.push(cv2);
      const pts: number[] = [];
      cv2.getPoints(30).forEach((p) => pts.push(p.x, p.y, p.z));
      const ln = mkL(pts, FGc, 0, { strip: true, dash: 0.25, gap: 0.2 });
      sc.add(ln);
      wLines.push(ln);
    }
    wPk = sph(0.17, Gc, 0);
    sc.add(wPk);
    wLab = S.label("Same money, passed around", "note");
  }
  const mF = new T.MeshBasicMaterial({ color: 0x1b201e, transparent: true, opacity: 0 });
  const mE = new T.LineBasicMaterial({ color: FGc, transparent: true, opacity: 0 });
  const mGF = new T.MeshBasicMaterial({ color: Gc, transparent: true, opacity: 0 });
  const mg = new T.BoxGeometry(0.36, 0.36, 0.36);
  const meg = new T.EdgesGeometry(mg);
  const minis: any[] = [];
  for (i = 0; i < 9; i++) {
    const mm = new T.Group();
    mm.add(new T.Mesh(mg, i === 4 ? mGF : mF));
    mm.add(new T.LineSegments(meg, mE));
    mm.visible = false;
    sc.add(mm);
    minis.push(mm);
  }
  const conn = mkL([0, 0, 0, 0, 1, 0], Gc, 0);
  conn.frustumCulled = false;
  sc.add(conn);
  const L = new T.Group();
  const R0 = cfg.lensR || 1.3;
  L.add(topMesh(new T.CircleGeometry(R0, 64), FGc, 0.05));
  L.add(topMesh(new T.RingGeometry(R0, R0 + 0.12, 64), FGc, 0.95));
  const progM: any = topMesh(new T.RingGeometry(R0 + 0.24, R0 + 0.33, 64, 1, 0, 0.001), Gc, 1);
  L.add(progM);
  let lastPr = -1;
  const hdd = R0 + 0.12 + 0.85;
  const hdl = topMesh(new T.PlaneGeometry(0.34, 1.7), FGc, 0.95);
  hdl.position.set(Math.cos(-Math.PI / 4) * hdd, Math.sin(-Math.PI / 4) * hdd, 0);
  hdl.rotation.z = Math.PI / 4;
  L.add(hdl);
  const capM = topMesh(new T.CircleGeometry(0.17, 24), FGc, 0.95);
  capM.position.set(Math.cos(-Math.PI / 4) * (hdd + 0.85), Math.sin(-Math.PI / 4) * (hdd + 0.85), 0);
  L.add(capM);
  sc.add(L);
  const cw = document.createElement("div");
  cw.className = "txw";
  S.lb.appendChild(cw);
  let cwKey: any = null;
  let cwIn: any = null;
  let cwRv: any[] = [];
  const peekLab = S.label("", "wal");
  const SC = cfg.scenes;
  const camR = new T.Vector3(13, 0, -6).normalize();
  const camO = new T.Vector3(6, 9, 13);
  function focusPos(f: string) {
    if (f === "hist") {
      const v = new T.Vector3();
      hist.forEach((b) => {
        v.x += b.x / hist.length;
        v.z += b.z / hist.length;
      });
      return v;
    }
    if (f === "wash") return wCenter.clone();
    if (keyB[f]) return new T.Vector3(keyB[f].x, 0, keyB[f].z);
    return new T.Vector3(0, 0, 0);
  }
  const camCache: any[] = SC.map((s: any) => {
    const F = focusPos(s.focus);
    const d = (s.dist || 1) * (cfg.distMul || 1);
    const look = F.clone().addScaledVector(camR, -(s.side || 0) * 5.5 * (s.dist || 1));
    look.y = s.lookY || 0.8;
    const pos = look.clone().add(camO.clone().multiplyScalar(d));
    return { pos, look };
  });
  let sCur = cfg.at();
  let wander: any = null;
  let wanderT = 0;
  let clickB: any = null;
  let clickT = 0;
  let clickO = 0;
  let popT = 0;
  let hoverB: any = null;
  const ray = new T.Raycaster();
  const plane = new T.Plane(new T.Vector3(0, 1, 0), -1);
  const hit = new T.Vector3();
  const ndc = new T.Vector2();
  let camP: any = null;
  const camL = new T.Vector3();
  const Yax = new T.Vector3(0, 1, 0);
  const lensP = new T.Vector3();
  const lensT = new T.Vector3();
  const hidden = new T.Vector3();
  function nearestBlock(p: any) {
    let best = null;
    let bd = 1e9;
    blocks.forEach((b) => {
      const d = (b.x - p.x) * (b.x - p.x) + (b.z - p.z) * (b.z - p.z);
      if (d < bd) {
        bd = d;
        best = b;
      }
    });
    return bd < 4 ? best : null;
  }
  function pickWander() {
    wPicks++;
    const c = blocks.filter((b) => Math.abs(b.x) < cols * SX * 0.32 && Math.abs(b.z) < rows * SZ * 0.34 && (!cfg.wanderPick || cfg.wanderPick(b)));
    wander = c[Math.floor(Math.random() * c.length)] || blocks[Math.floor(blocks.length / 2)];
    wanderT = 0;
    cfg.onPeek?.(wander);
    if (cfg.autoOpen && wPicks > 1 && wPicks % cfg.autoOpen === 0 && !clickB) {
      clickB = wander;
      clickT = 0;
      cfg.onClickBlock?.(wander, true);
    }
  }
  pickWander();
  lensP.copy(topOf(wander)).y += 1.6;
  S.onClick = (x: number, y: number) => {
    ndc.set((x / S.W) * 2 - 1, -(y / S.H) * 2 + 1);
    ray.setFromCamera(ndc, S.cam);
    plane.constant = -1;
    if (!ray.ray.intersectPlane(plane, hit)) return;
    const b = nearestBlock(hit);
    if (b) {
      clickB = b;
      clickT = 0;
      cfg.onClickBlock?.(b);
    }
  };
  function update(dt: number, now: number) {
    const k = inst ? 1 : 1 - Math.exp(-dt * (cfg.smooth || 12));
    const sT = cfg.at();
    sCur += (sT - sCur) * k;
    const user = S.pointer && now < S.userUntil && !S.dragging;
    let act: any = null;
    let actO = 0;
    let eHist = 0;
    let eWash = 0;
    let eWander = 0;
    SC.forEach((s: any, ix: number) => {
      const d = Math.abs(sCur - ix);
      const w = clamp(1 - d * 1.6, 0, 1);
      const o = clamp(1 - (d - 0.12) / 0.38, 0, 1);
      if (s.card && o > actO) {
        actO = o;
        act = s;
      }
      if (s.hist) eHist = Math.max(eHist, w);
      if (s.wash) eWash = Math.max(eWash, w);
      if (s.wander) eWander = Math.max(eWander, w);
    });
    if (clickB) {
      clickT += dt;
      clickO = clickT < 0.4 ? clickT / 0.4 : clickT < 3.6 ? 1 : Math.max(0, 1 - (clickT - 3.6) / 0.4);
      if (clickT > 4) {
        clickB = null;
        clickO = 0;
      }
    }
    const useClick = clickB && actO < 0.3;
    const openB = useClick ? clickB : act ? keyB[act.cardAt || act.card] : null;
    const openO = useClick ? clickO : actO;
    const openKey = useClick ? "G" + clickB.num : act ? act.card : null;
    wanderT += dt;
    if (wanderT > (cfg.wanderDur || 1.9) && !clickB) pickWander();
    hoverB = null;
    let hasHit = false;
    if (user) {
      ndc.set((S.pointer.x / S.W) * 2 - 1, -(S.pointer.y / S.H) * 2 + 1);
      ray.setFromCamera(ndc, S.cam);
      plane.constant = -1;
      if (ray.ray.intersectPlane(plane, hit)) {
        hasHit = true;
        hoverB = nearestBlock(hit);
      }
    }
    if (cfg.chain && !inst) {
      chainT += dt;
      if (chainT > cfg.chain) {
        chainT = 0;
        const rr = Math.floor(Math.random() * rows);
        if (!(openB && openB.r === rr) && rowA[rr] < 0) rowA[rr] = 0;
      }
      for (let ri = 0; ri < rows; ri++)
        if (rowA[ri] >= 0) {
          rowA[ri] += dt;
          if (rowA[ri] >= 0.75) {
            rowA[ri] = -1;
            blocks.forEach((b) => {
              if (b.r === ri) {
                b.c++;
                if (b.c >= cols) {
                  b.c = 0;
                  b.pop = 0;
                  b.num = ++topNum;
                }
              }
            });
          }
        }
    }
    popT += dt;
    if (popT > 2.3 && !inst) {
      popT = 0;
      const pb = blocks[Math.floor(Math.random() * blocks.length)];
      if (pb !== openB) pb.pop = 0;
    }
    blocks.forEach((b) => {
      let lt = 0;
      let em: any = eDim;
      let fm: any = fM;
      let sx = 1;
      let sy = b.h;
      let px = 0;
      let pz = 0;
      if (cfg.chain) b.x = X0 + b.c * SX + (rowA[b.r] >= 0 ? ease(rowA[b.r] / 0.75) * SX : 0);
      if (hasHit) {
        const dd = Math.hypot(b.x - hit.x, b.z - hit.z);
        lt += Math.max(0, 0.55 - dd * 0.11) + Math.max(0, Math.sin(S.t * 5 - dd * 1.3)) * 0.07 * Math.max(0, 1 - dd / 8);
      }
      if (openB && openB !== b && openO > 0) {
        const ddx = b.x - openB.x;
        const ddz = b.z - openB.z;
        const d2 = Math.hypot(ddx, ddz);
        if (d2 < 7.5 && d2 > 0) {
          const kk = (1 - d2 / 7.5) * 1.2 * openO;
          px = (ddx / d2) * kk;
          pz = (ddz / d2) * kk;
          lt -= 0.2 * kk;
        }
      }
      if (b === wander && eWander > 0.2) {
        lt = 0.35;
        em = eHi;
      }
      if (hist.indexOf(b) >= 0 && eHist > 0.25) {
        lt = Math.max(lt, 0.3 * eHist);
        em = eG;
      }
      if (wash.indexOf(b) >= 0 && eWash > 0.25) {
        lt = Math.max(lt, 0.3 * eWash);
        em = eHi;
      }
      if (b === hoverB) {
        lt = Math.max(lt, 0.45);
        em = em === eDim ? eHi : em;
      }
      if (b.pop >= 0) {
        b.pop += dt;
        sy = b.h * (0.15 + 0.85 * ease(Math.min(1, b.pop / 0.7)));
        em = eG;
        if (b.pop > 1.3) b.pop = -1;
      }
      if (b === openB && openO > 0) {
        lt = Math.max(lt, 1.3 * openO);
        sx = 1 + 0.25 * openO;
        if (openO > 0.15) {
          em = eG;
          fm = fG;
        }
      }
      b.lift += (lt - b.lift) * (inst ? 1 : 1 - Math.exp(-dt * 8));
      b.g.scale.set(sx, sy, sx);
      b.g.position.y = sy / 2 + b.lift;
      b.g.position.x = b.x + px;
      b.g.position.z = b.z + pz;
      if (b.e.material !== em) b.e.material = em;
      if (b.m.material !== fm) b.m.material = fm;
    });
    pks.forEach((p) => {
      const u = p.userData;
      u.x += dt * u.v;
      if (u.x > X0 + (cols - 1) * SX) u.x = X0;
      p.position.set(u.x, 0.25, Z0 + u.row * SZ);
    });
    const tot = eHist * hArcs.length;
    hArcs.forEach((tb, ix) => {
      const f = clamp(tot - ix, 0, 1);
      tb.geometry.setDrawRange(0, Math.floor(f * TS) * RS * 6);
    });
    wLines.forEach((l) => {
      l.material.opacity = 0.7 * eWash;
    });
    if (wPk) {
      wPk.material.opacity = eWash;
      const q = ((S.t * 0.35) % 1) * wCurves.length;
      const qi = Math.floor(q);
      wPk.position.copy(wCurves[qi].getPoint(q - qi));
      S.place(wLab, new T.Vector3(wCenter.x, 2.8, wCenter.z), eWash > 0.5);
    }
    const tp = openB ? topOf(openB) : null;
    minis.forEach((mm, ix) => {
      mm.visible = !!openB && openO > 0.02;
      if (!mm.visible || !tp) return;
      const gx = (ix % 3) - 1;
      const gz = Math.floor(ix / 3) - 1;
      const sp = 0.62 * openO;
      mm.position.set(tp.x + gx * sp, tp.y + 0.35 + 0.7 * openO + (ix === 4 ? 0.9 * openO : 0), tp.z + gz * sp);
      mm.rotation.y = S.t * 0.4;
    });
    mF.opacity = 0.95 * openO;
    mE.opacity = 0.6 * openO;
    mGF.opacity = openO;
    let anc: any = null;
    if (openB && openO > 0.02 && tp) {
      anc = tp.clone();
      anc.y += cfg.cardLift || 3.4;
      const a3 = conn.geometry.attributes.position;
      a3.setXYZ(0, tp.x, tp.y + 0.35 + 1.6 * openO, tp.z);
      a3.setXYZ(1, anc.x, anc.y, anc.z);
      a3.needsUpdate = true;
      conn.material.opacity = openO;
    } else conn.material.opacity = 0;
    if (anc) {
      if (cwKey !== openKey) {
        cwKey = openKey;
        cw.innerHTML = txnHtml(useClick ? genTxn(clickB) : (TXNS[openKey] as Txn), cfg.compact);
        cwIn = cw.firstChild;
        cwRv = [].slice.call(cw.querySelectorAll(".rv"));
      }
      const q2 = S.proj(anc);
      const cwW = cwIn.offsetWidth || 300;
      const cwH = cwIn.offsetHeight || 200;
      const ctop = typeof cfg.cardTop === "function" ? cfg.cardTop() : cfg.cardTop || 12;
      const px = clamp(q2.x, cwW / 2 + 12, Math.max(cwW / 2 + 12, S.W - cwW / 2 - 12));
      const py = clamp(q2.y, cwH + ctop + 6, Math.max(cwH + 18, S.H - 12));
      cw.style.transform = "translate(" + px.toFixed(1) + "px," + py.toFixed(1) + "px)";
      cwIn.style.opacity = openO.toFixed(3);
      cwIn.style.transform = "translate(-50%,calc(-100% - 6px)) scale(" + (0.86 + 0.14 * openO).toFixed(3) + ")";
      cwRv.forEach((el: any, ix: number) => {
        const on = openO > 0.3 + ix * (0.55 / cwRv.length);
        if (el._on !== on) {
          el._on = on;
          el.classList.toggle("on", on);
        }
      });
    } else if (cwIn) {
      cwIn.style.opacity = 0;
    }
    if (user) lensT.copy(hoverB ? topOf(hoverB) : hit).y += 1.6;
    else if (openB && openO > 0.05 && tp) lensT.copy(tp).y += 0.9;
    else lensT.copy(topOf(wander)).y += 1.6;
    lensP.lerp(lensT, inst ? 1 : 1 - Math.exp(-dt * (user ? 9 : 3.6)));
    const lpv = openB ? openO : user ? 0 : Math.min(1, wanderT / (cfg.wanderDur || 1.9));
    if (Math.abs(lpv - lastPr) > 0.008) {
      lastPr = lpv;
      progM.geometry.dispose();
      progM.geometry = new T.RingGeometry(R0 + 0.24, R0 + 0.33, 64, 1, Math.PI / 2 - Math.max(lpv, 0.001) * Math.PI * 2, Math.max(lpv, 0.001) * Math.PI * 2);
    }
    const pb2 = user ? hoverB : eWander > 0.4 && !(openB && openO > 0.1) ? wander : null;
    if (pb2) {
      if (peekLab._b !== pb2) {
        peekLab._b = pb2;
        peekLab.firstChild.textContent = "Block #" + pb2.num.toLocaleString("en-US") + " · " + (120 + (pb2.num % 180)) + " trades";
      }
      S.place(peekLab, topOf(pb2), true);
    } else S.place(peekLab, hidden, false);
    const ki = clamp(Math.floor(sCur), 0, SC.length - 1);
    const k2 = Math.min(ki + 1, SC.length - 1);
    const f = ease(clamp(sCur - ki, 0, 1));
    const pos = camCache[ki].pos.clone().lerp(camCache[k2].pos, f);
    const look = camCache[ki].look.clone().lerp(camCache[k2].look, f);
    const off = pos.sub(look).applyAxisAngle(Yax, S.yaw + (inst ? 0 : Math.sin(S.t * 0.12) * 0.12));
    const rad = off.length();
    off.y = clamp(off.y + S.pitch * rad * 0.7, 2, rad * 1.3);
    if (cfg.aspectFix && S.W / S.H < 1) off.multiplyScalar(1.45);
    const want = look.clone().add(off);
    if (!camP) {
      camP = want.clone();
      camL.copy(look);
    }
    const fc = inst ? 1 : 1 - Math.exp(-dt * 3.2);
    camP.lerp(want, fc);
    camL.lerp(look, fc);
    S.cam.position.copy(camP);
    S.cam.lookAt(camL);
    S.cam.updateMatrixWorld();
    L.position.copy(lensP);
    L.quaternion.copy(S.cam.quaternion);
  }
  S.start(update);
  return S;
}

export interface ChartEvent {
  /** Position along the day: 0..190 minutes after 09:00 (design timeline scale). */
  t: number;
  w: string;
  amt: string;
  sell?: boolean;
  label?: string;
}

/** TraceChart: token 3D view. `series` is the price path; `events` are wallet buys/sells. */
export function TraceChart(host: HTMLElement, opts: { series: number[]; events: ChartEvent[]; seed: number; heavy?: boolean }): Engine {
  const S = stage(host, { fov: 34, fogNear: 44, fogFar: 110 });
  const sc = S.scene;
  const ser: any[] = opts.series;
  const n = ser.length;
  const ev = opts.events;
  const r = rng(opts.seed + 5);
  let i: number;
  const mn = Math.min(...ser);
  const mx = Math.max(...ser);
  const X = (k: number) => -10 + (20 * k) / (n - 1);
  const Yv = (v: number) => 0.4 + ((v - mn) / (mx - mn || 1)) * 5.2;
  const I = (t: number) => Math.round(clamp(t / 190, 0, 1) * (n - 1));
  sc.add(dotFloor(-12, 12, -7, 5, 1));
  const ar: number[] = [];
  for (i = 0; i < n - 1; i++) {
    const x1 = X(i);
    const x2 = X(i + 1);
    const y1 = Yv(ser[i]);
    const y2 = Yv(ser[i + 1]);
    ar.push(x1, 0, 0, x1, y1, 0, x2, y2, 0, x1, 0, 0, x2, y2, 0, x2, 0, 0);
  }
  sc.add(new T.Mesh(geo(ar), new T.MeshBasicMaterial({ color: Gc, transparent: true, opacity: 0.1, side: T.DoubleSide })));
  const cc = new T.CatmullRomCurve3(ser.map((v, k) => new T.Vector3(X(k), Yv(v), 0)));
  sc.add(new T.Mesh(new T.TubeGeometry(cc, 220, 0.07, 8, false), new T.MeshBasicMaterial({ color: Gc })));
  sc.add(mkL([X(0), Yv(ser[0]), 0, X(n - 1), Yv(ser[0]), 0], FGc, 0.3, { dash: 0.25, gap: 0.25 }));
  const vb = new T.BoxGeometry(0.26, 1, 0.26);
  const vm = new T.MeshBasicMaterial({ color: FGc, transparent: true, opacity: 0.14 });
  for (i = 0; i < n; i += 2) {
    const h = (0.2 + r() * 0.5) * (i > n * 0.34 ? 2.1 : 1) * (opts.heavy ? 1.6 : 1);
    const b = new T.Mesh(vb, vm);
    b.scale.y = h;
    b.position.set(X(i), h / 2, 3.4);
    sc.add(b);
  }
  const lanes: Record<string, number> = {};
  let laneN = 0;
  const marks: any[] = [];
  ev.forEach((e) => {
    if (lanes[e.w] === undefined) lanes[e.w] = -1.8 - laneN++ * 1.5;
    const z = lanes[e.w] as number;
    const k = I(e.t);
    const x = X(k);
    const y = Yv(ser[k]);
    const col = e.sell ? FGc : Gc;
    const c = sph(0.3, col, 1);
    c.position.set(x, 0.3, z);
    sc.add(c);
    sc.add(new T.Mesh(new T.TubeGeometry(new T.LineCurve3(new T.Vector3(x, 0.6, z), new T.Vector3(x, y, z)), 4, 0.04, 6, false), new T.MeshBasicMaterial({ color: col, transparent: true, opacity: e.sell ? 0.6 : 1 })));
    sc.add(mkL([x, y, z, x, y, 0], FGc, 0.35, { dash: 0.15, gap: 0.15 }));
    const m = sph(0.17, col, 1);
    m.position.set(x, y, z);
    sc.add(m);
    marks.push({ p: new T.Vector3(x, y, z), lab: S.label(e.label || e.w + (e.sell ? " sold " : " bought ") + e.amt + " at " + hm(e.t), "tip") });
  });
  const laneLabs = Object.keys(lanes).map((w) => [S.label(w, "wal named"), new T.Vector3(-11.2, 0.3, lanes[w])]);
  const tl = [0, 60, 120, 180].map((t) => [S.label(hm(t), "ax"), new T.Vector3(X(I(t)), 0, 5.4)]);
  const pl = [S.label(pf(ser[n - 1]) + " now", "big"), new T.Vector3(X(n - 1), Yv(ser[n - 1]) + 0.7, 0)];
  const vl = [S.label("Trading volume", "wal"), new T.Vector3(-11, 0.2, 3.4)];
  const tgt = new T.Vector3(0, 2.2, -0.6);
  function update(_dt: number, now: number) {
    const idle = !S.dragging && !(S.pointer && now < S.userUntil);
    const yaw = -0.62 + S.yaw + (idle ? Math.sin(S.t * 0.25) * 0.14 : 0);
    const pitch = clamp(0.36 + S.pitch, 0.08, 1.1);
    const rad = S.W / S.H < 1.3 ? 40 : 31;
    S.cam.position.set(tgt.x + rad * Math.cos(pitch) * Math.sin(yaw), tgt.y + rad * Math.sin(pitch), tgt.z + rad * Math.cos(pitch) * Math.cos(yaw));
    S.cam.lookAt(tgt);
    S.cam.updateMatrixWorld();
    let hov: any = null;
    let bd = 30;
    if (S.pointer) {
      marks.forEach((m) => {
        const q = S.proj(m.p);
        const d = Math.hypot(q.x - S.pointer.x, q.y - S.pointer.y);
        if (d < bd) {
          bd = d;
          hov = m;
        }
      });
    }
    marks.forEach((m) => S.place(m.lab, m.p, m === hov));
    laneLabs.forEach((l) => S.place(l[0], l[1], true));
    tl.forEach((l) => S.place(l[0], l[1], true));
    S.place(pl[0], pl[1], true);
    S.place(vl[0], vl[1], true);
  }
  S.onClick = () => undefined;
  S.start(update);
  return S;
}

/* ---------------- static SVG fallbacks ---------------- */

function isoXY(x: number, y: number, z: number, o: any) {
  return [o.cx + (x - z) * 0.866 * o.s, o.cy + (x + z) * 0.5 * o.s - y * o.s];
}
function isoPt(x: number, y: number, z: number, o: any) {
  const p = isoXY(x, y, z, o);
  return p[0].toFixed(1) + " " + p[1].toFixed(1);
}

export function isoField(w: number, h: number): string {
  const o = { cx: w / 2, cy: h * 0.58, s: Math.min(w, h) / 24 };
  const r = rng(5);
  const list: any[] = [];
  let out = "";
  for (let i = 0; i < 7; i++) for (let j = 0; j < 7; j++) list.push({ x: (j - 3) * 2.4, z: (i - 3) * 2.7, h: 0.7 + r() * 0.6, key: i === 3 && j === 3 });
  list.sort((a, b) => a.x + a.z - (b.x + b.z));
  function cube(x: number, z: number, hh: number, lift: number, st: string, fill: string) {
    const s = 0.65;
    const y0 = lift;
    const y1 = lift + hh;
    const c: any[] = [[x - s, y0, z - s], [x + s, y0, z - s], [x + s, y0, z + s], [x - s, y0, z + s], [x - s, y1, z - s], [x + s, y1, z - s], [x + s, y1, z + s], [x - s, y1, z + s]].map((q) => isoXY(q[0]!, q[1]!, q[2]!, o));
    const poly = (ix: number[]) => '<path d="M' + ix.map((k) => c[k][0].toFixed(1) + " " + c[k][1].toFixed(1)).join("L") + 'Z" fill="' + fill + '" stroke="' + st + '" stroke-width="1"/>';
    return poly([1, 2, 6, 5]) + poly([2, 3, 7, 6]) + poly([4, 5, 6, 7]);
  }
  list.forEach((b) => {
    out += b.key ? cube(b.x, b.z, b.h, 1.3, "#4AE38F", "#163323") : cube(b.x, b.z, b.h, 0, "rgba(237,242,239,.3)", "#131816");
  });
  return '<svg viewBox="0 0 ' + w + " " + h + '" preserveAspectRatio="xMidYMid slice" aria-hidden="true">' + out + "</svg>";
}

export function isoChart(series: number[], events: ChartEvent[], w: number, h: number): string {
  const o = { cx: w * 0.5, cy: h * 0.56, s: Math.min(w / 24, h / 13) };
  const n = series.length;
  const mn = Math.min(...series);
  const mx = Math.max(...series);
  let dots = "";
  const X = (i: number) => -8 + (16 * i) / (n - 1);
  const Y = (v: number) => 0.3 + ((v - mn) / (mx - mn || 1)) * 4.5;
  for (let i = -9; i <= 9; i++)
    for (let j = -4; j <= 4; j++) {
      const p = isoXY(i, 0, j, o);
      dots += '<circle cx="' + p[0].toFixed(1) + '" cy="' + p[1].toFixed(1) + '" r=".9"/>';
    }
  const top = series.map((v, i) => isoPt(X(i), Y(v), 0, o));
  const area = "M" + isoPt(X(0), 0, 0, o) + "L" + top.join("L") + "L" + isoPt(X(n - 1), 0, 0, o) + "Z";
  let ev = "";
  events.forEach((e, k) => {
    const i = Math.round((clamp(e.t, 0, 190) / 190) * (n - 1));
    const z = -1.6 - k * 1.1;
    const a = isoXY(X(i), 0.25, z, o);
    const b = isoXY(X(i), Y(series[i]!), z, o);
    ev += '<line x1="' + a[0] + '" y1="' + a[1] + '" x2="' + b[0] + '" y2="' + b[1] + '" stroke="#4AE38F" stroke-width="2" stroke-linecap="round"/><circle cx="' + a[0].toFixed(1) + '" cy="' + a[1].toFixed(1) + '" r="' + (o.s * 0.28).toFixed(1) + '" fill="#4AE38F"/>';
  });
  return '<svg viewBox="0 0 ' + w + " " + h + '" preserveAspectRatio="xMidYMid meet" aria-hidden="true"><g fill="rgba(237,242,239,.2)">' + dots + '</g><path d="' + area + '" fill="rgba(74,227,143,.12)"/>' + ev + '<path d="M' + top.join("L") + '" fill="none" stroke="#4AE38F" stroke-width="2.5" stroke-linejoin="round"/></svg>';
}
