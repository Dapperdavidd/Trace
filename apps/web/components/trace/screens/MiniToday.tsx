"use client";

import { D, RANGE } from "@/lib/trace/example";
import { sgn, walk } from "@/lib/trace/viz";
import { Chart2, Ic, Spark, Tk } from "../ui";

/** Static Today screen for the landing page phone. Example data, no network. */
export function MiniToday() {
  return (
    <div className="scr s-today">
      <div className="hello">
        <div>
          <div className="hrow2">
            <span className="lab">Sat 07 Nov · 12:10</span>
            <span className="avb">AB</span>
          </div>
          <h1>Good afternoon.</h1>
          <p>3 signals today. Your copies are up 18.7% this week.</p>
        </div>
        <div className="qa">
          <span className="qb">
            <Ic id="grid" />
            <span>Discover</span>
          </span>
          <span className="qb">
            <Ic id="flask" />
            <span>Test</span>
          </span>
          <span className="qb">
            <Ic id="swap" />
            <span>Swap</span>
          </span>
          <span className="qb pri">
            <Ic id="plus" />
            <span>Buy</span>
          </span>
        </div>
      </div>
      <div className="tg">
        <div className="pn">
          <div className="pn-h">
            <span className="lab">Portfolio</span>
          </div>
          <div className="vrow">
            <span className="big n">$2,840.25</span>
            <span className="mono g">{RANGE["1D"]!.d}</span>
          </div>
          <Chart2 sets={[{ v: walk(RANGE["1D"]!.seed, 72, 0.36) }]} />
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">Today&apos;s signals</span>
          </div>
          <div>
            {["NEURA", "SOLX", "BETA"].map((s) => {
              const d = D.coins[s]!;
              return (
                <div className="li" key={s}>
                  <Tk s={s} on={s === "NEURA"} />
                  <div className="grow">
                    <div className="nm">{s}</div>
                    <div className="sub">{d.why}</div>
                  </div>
                  <Spark seed={d.seed} />
                  <span className="mono g">{sgn(d.ch)}%</span>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
