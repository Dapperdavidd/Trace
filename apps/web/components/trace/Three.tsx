"use client";

import { useEffect, useRef, type CSSProperties } from "react";
import { Field, TraceChart, has3D, isoChart, isoField, type ChartEvent, type Engine, type FieldConfig } from "@/lib/trace/engine";

/** Mounts the block field. `config` is read once on mount, like the design's Frame.render. */
export function FieldView({ config, className, id, style }: { config: () => FieldConfig; className?: string; id?: string; style?: CSSProperties }) {
  const ref = useRef<HTMLDivElement>(null);
  const cfg = useRef(config);
  cfg.current = config;
  useEffect(() => {
    const host = ref.current;
    if (!host) return;
    let eng: Engine | null = null;
    if (has3D()) {
      try {
        eng = Field(host, cfg.current());
      } catch {
        host.innerHTML = isoField(1200, 800);
      }
    } else {
      const b = host.getBoundingClientRect();
      host.innerHTML = isoField(Math.round(b.width) || 1200, Math.round(b.height) || 800);
    }
    return () => {
      if (eng) eng.dispose();
      else host.innerHTML = "";
    };
  }, []);
  return <div ref={ref} className={className} id={id} style={style} />;
}

export function TraceChartView({ series, events, seed, heavy, className = "tv" }: { series: number[]; events: ChartEvent[]; seed: number; heavy?: boolean; className?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  const key = JSON.stringify([series.slice(0, 3), series.length, series[series.length - 1], events, seed]);
  useEffect(() => {
    const host = ref.current;
    if (!host || series.length < 2) return;
    let eng: Engine | null = null;
    if (has3D()) {
      try {
        eng = TraceChart(host, { series, events, seed, heavy });
      } catch {
        eng = null;
      }
    }
    if (!eng) {
      const b = host.getBoundingClientRect();
      host.innerHTML = isoChart(series, events, Math.round(b.width) || 600, Math.round(b.height) || 360);
    }
    return () => {
      if (eng) eng.dispose();
      else host.innerHTML = "";
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return <div ref={ref} className={className} />;
}
