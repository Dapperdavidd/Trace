"use client";

import { useRouter } from "next/navigation";
import { useEffect, useState, type ReactNode } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { AgentConnectionCreated, AlertPreferences } from "@/lib/api/types";
import { useWallet, yoctoToNear } from "@/lib/trace/wallet";
import { errText, savedShadows } from "@/lib/trace/near";
import { shortAddr } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { initials } from "../AppFrame";
import { Av, Hd, Help, Ic, Lk, Row, Seg, Sw, Tk } from "../ui";

function MRow({ href, onClick, icon, t, sub, extra }: { href?: string; onClick?: () => void; icon: string; t: string; sub?: string; extra?: ReactNode }) {
  return (
    <Row href={href} onClick={onClick}>
      <span className="tk">
        <Ic id={icon} />
      </span>
      <div className="grow">
        <div className="nm">{t}</div>
        {sub ? <div className="sub">{sub}</div> : null}
      </div>
      {extra}
      <Ic id="arr" />
    </Row>
  );
}

function SignInCard({ next, why }: { next: string; why: string }) {
  const router = useRouter();
  return (
    <div className="pn">
      <span className="lab">You are not signed in</span>
      <p className="sub" style={{ whiteSpace: "normal", margin: 0 }}>
        {why}
      </p>
      <button className="bfill" type="button" onClick={() => router.push(`/welcome/?next=${encodeURIComponent(next)}`)}>
        Sign in with a passkey
      </button>
    </div>
  );
}

/* =============================== Profile (me) =============================== */

export function MeScreen() {
  const router = useRouter();
  const token = useAuthToken();
  const { star, prefs, toast } = useApp();
  const me = useApi(() => api.me(), [token], !!token);
  const watch = useApi(() => api.watchlists(), [token], !!token);
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const prefsLive = useApi(() => api.alertPreferences(), [token], !!token);
  const coins = Object.keys(star).filter((k) => star[k] && k[0] !== "w" && k.includes(":")).length;
  const follows = watch.data?.items.length ?? 0;
  const onCount = prefsLive.data ? Object.values(prefsLive.data).filter(Boolean).length : 0;
  const [shadowN, setShadowN] = useState(0);
  useEffect(() => setShadowN(savedShadows().length), []);
  const name = me.data?.name || me.data?.email || "Your account";
  const { network, wallet, balance } = useWallet();

  return (
    <div className="scr s-me">
      <Hd
        title="Profile"
        crumb={token ? "Signed in with a passkey" : "Browsing as a guest"}
        back
        right={
          <button
            className="hb"
            type="button"
            aria-label="Share trace"
            onClick={() => {
              navigator.clipboard?.writeText(window.location.origin).catch(() => undefined);
              toast("trace link copied");
            }}
          >
            <Ic id="share" />
          </button>
        }
      />
      <div className="mgrid2">
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          {token ? (
            <div className="pn">
              <div className="phead">
                <span className="av l">{initials(name) || "T"}</span>
                <div style={{ display: "grid", gap: 4, minWidth: 0 }}>
                  <div className="nm" style={{ fontSize: 20 }}>
                    {name}
                  </div>
                  <div className="sub">{me.data?.email ?? "Passkey account"}</div>
                  <span className="tag g" style={{ justifySelf: "start" }}>
                    Free plan
                  </span>
                </div>
              </div>
              <div className="stat">
                <div>
                  <span className="lab">Shadow tests</span>
                  <b>{shadowN}</b>
                </div>
                <div>
                  <span className="lab">Coins watched</span>
                  <b>{coins}</b>
                </div>
                <div>
                  <span className="lab">Copying</span>
                  <b>{mandates.data?.items.filter((m) => m.status === "anchored").length ?? 0} wallets</b>
                </div>
                <div>
                  <span className="lab">Following</span>
                  <b>{follows} wallets</b>
                </div>
              </div>
            </div>
          ) : (
            <SignInCard next="/app/me/" why="Create an account with a passkey to follow wallets, get alerts and sign copy limits on NEAR." />
          )}
          <div className="pn">
            <div className="pn-h">
              <span className="lab">Your wallet</span>
              <Lk href="/app/wallet/">Wallet</Lk>
            </div>
            {wallet ? (
              <div className="kvl">
                <div>
                  <span>Network</span>
                  <span className="mono">{network}</span>
                </div>
                <div>
                  <span>Account</span>
                  <span className="mono">{shortAddr(wallet.accountId)}</span>
                </div>
                <div>
                  <span>Balance</span>
                  <span className="mono">{balance ? `${yoctoToNear(balance.available, 3)} NEAR` : "…"}</span>
                </div>
              </div>
            ) : (
              <span className="sub" style={{ whiteSpace: "normal" }}>
                No wallet on {network} yet. Create one from the Wallet tab.
              </span>
            )}
          </div>
        </div>
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">Account</span>
            <div>
              <MRow href="/app/following/" icon="star" t="Following and watchlist" sub={`${follows} wallets · ${coins} ${coins === 1 ? "coin" : "coins"}`} />
              <MRow href="/app/notifs/" icon="bell" t="Notifications" sub={token ? `${onCount} kinds of alerts on` : "Sign in to choose alerts"} />
              <MRow href="/app/security/" icon="lock" t="Security and devices" sub="Passkey, devices, connected agents" extra={token ? <span className="tag g">Passkey</span> : <span className="tag w">Sign in</span>} />
              <MRow href="/app/near/" icon="key" t="NEAR agent" sub="Copy limits you sign on NEAR" />
              <MRow href="/app/prefs/" icon="globe" t="Preferences" sub={`${prefs.cur} · ${prefs.lang}`} />
            </div>
          </div>
          <div className="pn">
            <span className="lab">More</span>
            <div>
              <MRow
                onClick={() => {
                  navigator.clipboard?.writeText(window.location.origin).catch(() => undefined);
                  toast("Invite link copied");
                }}
                icon="gift"
                t="Invite friends"
                sub="Share trace with someone who trades"
              />
              <MRow onClick={() => window.open("https://github.com/Dapperdavidd/Trace", "_blank", "noopener")} icon="help" t="Help and source" sub="Guides, and the open source code" />
              {token ? (
                <MRow
                  onClick={async () => {
                    await api.logout().catch(() => undefined);
                    toast("Signed out");
                    router.push("/");
                  }}
                  icon="out"
                  t="Sign out"
                  sub="Your passkey stays on this device"
                />
              ) : (
                <MRow href="/welcome/" icon="user" t="Sign in" sub="Use a passkey. No password." />
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* =============================== Notifications =============================== */

const N: Array<[keyof AlertPreferences | "brief", string, string]> = [
  ["wallet_buy", "Wallets you follow buy something", "Within a minute of the trade"],
  ["wallet_sell", "Wallets you follow sell", "So you know when smart money leaves"],
  ["risk_event", "Safety warnings", "When a coin you watch fails a check"],
  ["brief", "Daily briefing", "A short summary every morning"],
  ["signal", "Signals", "When trace publishes a research signal"],
];

export function NotifsScreen() {
  const token = useAuthToken();
  const { prefs, setPrefs, toast } = useApp();
  const p = useApi(() => api.alertPreferences(), [token], !!token);
  const [local, setLocal] = useState<AlertPreferences | null>(null);
  const [brief, setBrief] = useState(true);
  useEffect(() => {
    if (p.data) setLocal(p.data);
  }, [p.data]);
  const toggle = async (k: keyof AlertPreferences) => {
    if (!local) return;
    const next = { ...local, [k]: !local[k] };
    setLocal(next);
    try {
      await api.patchAlertPreferences({ [k]: next[k] });
    } catch (e) {
      setLocal(local);
      toast(errText(e));
    }
  };
  const count = (local ? Object.values(local).filter(Boolean).length : 0) + (brief ? 1 : 0);

  return (
    <div className="scr s-nt">
      <Hd title="Notifications" crumb={token ? `${count} of ${N.length} turned on` : "Sign in to choose alerts"} back />
      <div className="mgrid2">
        {token ? (
          <div className="pn">
            <div>
              {N.map(([k, t, s]) => {
                const on = k === "brief" ? brief : !!local?.[k];
                return (
                  <div key={k}>
                    <div className="grow2">
                      <div>
                        <b style={{ fontWeight: 500 }}>{t}</b>
                        <div className="sub" style={{ whiteSpace: "normal" }}>
                          {s}
                        </div>
                      </div>
                      <Sw on={on} label={t} onClick={() => (k === "brief" ? setBrief(!brief) : toggle(k))} />
                    </div>
                    {k === "brief" && on ? (
                      <div style={{ padding: "0 0 10px" }}>
                        <Seg
                          cls="box"
                          opts={[
                            ["07:00", "7:00"],
                            ["08:00", "8:00"],
                            ["09:00", "9:00"],
                            ["12:00", "Noon"],
                          ]}
                          cur={prefs.bt}
                          onPick={(bt) => {
                            setPrefs({ bt });
                            toast(`Daily briefing at ${bt}`);
                          }}
                        />
                      </div>
                    ) : null}
                  </div>
                );
              })}
            </div>
          </div>
        ) : (
          <SignInCard next="/app/notifs/" why="Alerts are tied to your account and the wallets you follow." />
        )}
        <div style={{ display: "grid", gap: 12, alignContent: "start" }}>
          <Help icon="bell">Alerts come from finalized trades only, so a trade that gets reorganised away never pings you.</Help>
          <div className="pn">
            <span className="lab">Preview</span>
            <div className="npv">
              <span className="tk on">t</span>
              <div className="grow">
                <b style={{ fontWeight: 500 }}>0xSun just bought NEURA</b>
                <div className="sub" style={{ whiteSpace: "normal" }}>
                  $18K at 09:24. It is right 78% of the time. Tap to see the trade.
                </div>
              </div>
              <span className="sub">now</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* =============================== Security =============================== */

export function SecurityScreen() {
  const router = useRouter();
  const token = useAuthToken();
  const { prefs, setPrefs, toast } = useApp();
  const sessions = useApi(() => api.sessions(), [token], !!token);
  const agents = useApi(() => api.agentConnections(), [token], !!token);
  const [newToken, setNewToken] = useState<AgentConnectionCreated | null>(null);
  const [busy, setBusy] = useState(false);

  const addAgent = async () => {
    setBusy(true);
    try {
      const c = await api.createAgentConnection({ name: "My agent", scopes: ["observe", "propose"], expires_in_days: 30 });
      setNewToken(c);
      agents.reload();
    } catch (e) {
      toast(errText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="scr s-sec">
      <Hd title="Security and devices" crumb="trace never holds your keys" back />
      <div className="mgrid2">
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          {token ? (
            <div className="secok">
              <Ic id="key" />
              <div className="grow">
                <b>Signed in with a passkey</b>
                <p>Your face, fingerprint or device PIN unlocks trace. There is no password to steal, and trace only stores a public key.</p>
              </div>
            </div>
          ) : (
            <div className="secwarn">
              <span className="tk">
                <Ic id="key" />
              </span>
              <div className="grow" style={{ display: "grid", gap: 8 }}>
                <b>You are not signed in</b>
                <p>Create a passkey to keep your follows, alerts and copy limits. It takes about 20 seconds.</p>
                <button className="bfill sm" style={{ justifySelf: "start" }} type="button" onClick={() => router.push("/welcome/?next=/app/security/")}>
                  Create a passkey
                </button>
              </div>
            </div>
          )}
          <div className="pn">
            <div className="grow2">
              <div>
                <b style={{ fontWeight: 500 }}>Passkey sign-in</b>
                <div className="sub" style={{ whiteSpace: "normal" }}>
                  Needed to open your account on a new device
                </div>
              </div>
              <span className={`tag ${token ? "g" : ""}`}>{token ? "On" : "Off"}</span>
            </div>
            <div className="grow2">
              <div>
                <b style={{ fontWeight: 500 }}>Ask before every signature</b>
                <div className="sub" style={{ whiteSpace: "normal" }}>
                  Your NEAR wallet shows the limits and asks you first. trace cannot skip this.
                </div>
              </div>
              <span className="tag g">Always</span>
            </div>
          </div>
          <div className="pn">
            <span className="lab">Daily spending limit</span>
            <Seg
              cls="box"
              opts={[
                [100, "$100"],
                [250, "$250"],
                [500, "$500"],
                [0, "No limit"],
              ]}
              cur={prefs.lim}
              onPick={(lim) => setPrefs({ lim })}
            />
            <span className="sub" style={{ whiteSpace: "normal" }}>
              {prefs.lim ? `New copy mandates stop for the day at $${prefs.lim}. NEAR enforces it.` : "New mandates use five times the amount per trade as the daily cap."}
            </span>
          </div>
        </div>
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">Connected agents</span>
            {token ? (
              <>
                <div>
                  {(agents.data?.items ?? []).filter((a) => a.status === "active").length ? (
                    agents.data?.items
                      .filter((a) => a.status === "active")
                      .map((a) => (
                        <div className="grow2" key={a.id}>
                          <div>
                            <b style={{ fontWeight: 500 }}>{a.name}</b>
                            <div className="sub">
                              {a.scopes.join(" and ")} · until {new Date(a.expires_at).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}
                            </div>
                          </div>
                          <button
                            className="bout sm"
                            type="button"
                            onClick={async () => {
                              await api.revokeAgentConnection(a.id).catch((e) => toast(errText(e)));
                              agents.reload();
                              toast(`${a.name} disconnected`);
                            }}
                          >
                            Remove
                          </button>
                        </div>
                      ))
                  ) : (
                    <p className="sub" style={{ margin: 0 }}>
                      No agents connected.
                    </p>
                  )}
                </div>
                {newToken ? (
                  <div style={{ display: "grid", gap: 6 }}>
                    <span className="sub" style={{ whiteSpace: "normal" }}>
                      Copy this token now. It is shown once.
                    </span>
                    <code className="hash">{newToken.token}</code>
                  </div>
                ) : null}
                <button className="bout" type="button" onClick={addAgent} disabled={busy}>
                  {busy ? "Connecting" : "Connect an agent"}
                </button>
                <span className="sub" style={{ whiteSpace: "normal" }}>
                  Agents can read and propose. They can never execute. Your NEAR mandate decides.
                </span>
              </>
            ) : (
              <p className="sub" style={{ margin: 0 }}>
                Sign in to connect an agent.
              </p>
            )}
          </div>
          <div className="pn">
            <span className="lab">Signed-in devices</span>
            {token ? (
              <>
                <div>
                  {(sessions.data?.items ?? []).map((s) => (
                    <div className="grow2" key={s.id}>
                      <div>
                        <b style={{ fontWeight: 500 }}>{s.device_label || "Device"}</b>
                        <div className="sub">
                          {s.current ? "This device · " : ""}since {new Date(s.created_at).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}
                        </div>
                      </div>
                      {!s.current ? (
                        <button
                          className="bout sm"
                          type="button"
                          onClick={async () => {
                            await api.revokeSession(s.id).catch((e) => toast(errText(e)));
                            sessions.reload();
                          }}
                        >
                          Sign out
                        </button>
                      ) : (
                        <span className="tag g">Now</span>
                      )}
                    </div>
                  ))}
                </div>
                <button
                  className="bout"
                  type="button"
                  onClick={async () => {
                    await api.logoutAll().catch(() => undefined);
                    toast("Signed out everywhere");
                    router.push("/");
                  }}
                >
                  Sign out everywhere
                </button>
                <button
                  className="blink"
                  type="button"
                  style={{ justifySelf: "start", padding: 0, height: 36 }}
                  onClick={async () => {
                    if (!window.confirm("Delete your trace account? Your follows, alerts, shadow tests and mandate drafts are removed. Mandates already signed on NEAR stay on NEAR until they expire.")) return;
                    try {
                      await api.deleteAccount();
                      toast("Account deleted");
                      router.push("/");
                    } catch (e) {
                      toast(errText(e));
                    }
                  }}
                >
                  Delete account
                </button>
              </>
            ) : (
              <p className="sub" style={{ margin: 0 }}>
                No devices. You are browsing as a guest.
              </p>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

/* =============================== Following =============================== */

export function FollowingScreen() {
  const router = useRouter();
  const token = useAuthToken();
  const { star, toggleStar, toast } = useApp();
  const watch = useApi(() => api.watchlists(), [token], !!token);
  const [fl, setFl] = useState("w");
  const items = watch.data?.items ?? [];
  const coinKeys = Object.keys(star).filter((k) => star[k] && k[0] !== "w" && k.includes(":"));

  return (
    <div className="scr s-fl">
      <Hd title="Following" crumb="Wallets you follow and coins you watch" back />
      <Seg
        cls="box"
        opts={[
          ["w", `Wallets · ${token ? items.length : 0}`],
          ["c", "Coins"],
        ]}
        cur={fl}
        onPick={setFl}
      />
      <div className="pn">
        <div>
          {fl === "w" ? (
            !token ? (
              <p className="sub" style={{ padding: "12px 0", whiteSpace: "normal" }}>
                Sign in to follow wallets and get an alert when they trade.
              </p>
            ) : items.length ? (
              items.map((it) => (
                <Row key={it.id} href={`/app/profile/?chain=${it.wallet.chain}&address=${encodeURIComponent(it.wallet.address)}`}>
                  <Av t={(it.wallet.alias || it.wallet.address.replace(/^0x/, "")).slice(0, 2).toUpperCase()} />
                  <div className="grow">
                    <div className="nm">{it.wallet.alias || shortAddr(it.wallet.address)}</div>
                    <div className="sub">
                      {it.wallet.chain} · {it.status.replaceAll("_", " ")}
                    </div>
                  </div>
                  <button
                    className="bout sm"
                    type="button"
                    onClick={async () => {
                      await api.deleteWatchlist(it.id).catch((e) => toast(errText(e)));
                      watch.reload();
                      toast("Unfollowed");
                    }}
                  >
                    Unfollow
                  </button>
                </Row>
              ))
            ) : (
              <p className="sub" style={{ padding: "12px 0" }}>
                You are not following anyone. Find wallets in Smart wallets.
              </p>
            )
          ) : coinKeys.length ? (
            coinKeys.map((k) => {
              const [chain, address] = k.includes(":") ? (k.split(":") as [string, string]) : ["", ""];
              if (!address) return null;
              return (
                <Row key={k} href={`/app/token/?chain=${chain}&address=${encodeURIComponent(address)}`}>
                  <Tk s={shortAddr(address)} on />
                  <div className="grow">
                    <div className="nm">{shortAddr(address)}</div>
                    <div className="sub">{chain}</div>
                  </div>
                  <button
                    className="bout sm"
                    type="button"
                    onClick={() => {
                      toggleStar(k);
                      toast("Removed from watchlist");
                    }}
                  >
                    Remove
                  </button>
                </Row>
              );
            })
          ) : (
            <p className="sub" style={{ padding: "12px 0", whiteSpace: "normal" }}>
              No coins yet. Tap the star on any coin to watch it.
            </p>
          )}
        </div>
      </div>
      <button className="bout" type="button" onClick={() => router.push("/app/wallets/")}>
        Find more wallets
      </button>
    </div>
  );
}

/* =============================== Preferences =============================== */

export function PrefsScreen() {
  const { prefs, setPrefs, toast } = useApp();
  const L = ["English", "Français", "Português", "Yorùbá"];
  return (
    <div className="scr s-pf">
      <Hd title="Preferences" crumb="How trace looks and talks to you" back />
      <div className="mgrid2">
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">Show amounts in</span>
            <Seg
              cls="box"
              opts={["USD", "NGN", "GBP", "EUR", "THB"].map((c) => [c, c] as [string, string])}
              cur={prefs.cur}
              onPick={(cur) => {
                setPrefs({ cur });
                toast(cur === "USD" ? "Amounts shown in USD" : `${cur} display is coming soon. Amounts stay in USD for now.`);
              }}
            />
            <span className="sub" style={{ whiteSpace: "normal" }}>
              Prices still trade in crypto. This only changes how amounts are shown.
            </span>
          </div>
          <div className="pn">
            <span className="lab">Language</span>
            <div>
              {L.map((l) => (
                <button
                  key={l}
                  type="button"
                  className="grow2 radio"
                  style={{ width: "100%", textAlign: "left" }}
                  onClick={() => {
                    setPrefs({ lang: l });
                    if (l !== "English") toast(`${l} is coming soon`);
                  }}
                >
                  <span>{l}</span>
                  <i className={`rd${prefs.lang === l ? " on" : ""}`} />
                </button>
              ))}
            </div>
          </div>
        </div>
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <div className="grow2">
              <div>
                <b style={{ fontWeight: 500 }}>Networks</b>
                <div className="sub">Ethereum and Solana, with NEAR for permissions</div>
              </div>
              <span className="tag">More soon</span>
            </div>
            <div className="grow2">
              <div>
                <b style={{ fontWeight: 500 }}>Hide tiny balances</b>
                <div className="sub" style={{ whiteSpace: "normal" }}>
                  Coins worth less than $1
                </div>
              </div>
              <Sw on={prefs.sb} label="Hide tiny balances" onClick={() => setPrefs({ sb: !prefs.sb })} />
            </div>
            <div className="grow2">
              <div>
                <b style={{ fontWeight: 500 }}>Appearance</b>
                <div className="sub">Dark</div>
              </div>
              <span className="tag">Only dark for now</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
