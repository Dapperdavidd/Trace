/* Icon sprite ported verbatim from design/trace-redesign-v11.html. */
const SPRITE = `<symbol id="lens" viewBox="0 0 24 24"><circle cx="10" cy="10" r="6.5"/><path class="h" d="M15 15l6 6"/><path d="M7 10h1.5l1-2 1.5 4 1-2H14" class="h"/></symbol>
  <symbol id="arr" viewBox="0 0 16 16"><path d="M3 8h10M9 4l4 4-4 4"/></symbol>
  <symbol id="back" viewBox="0 0 16 16"><path d="M13 8H3M7 4 3 8l4 4"/></symbol>
  <symbol id="srch" viewBox="0 0 16 16"><circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5 14 14"/></symbol>
  <symbol id="bell" viewBox="0 0 16 16"><path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3z"/><path d="M6.5 14.5h3"/></symbol>
  <symbol id="home" viewBox="0 0 16 16"><path d="M2.5 7 8 2.5 13.5 7v6.5h-11z"/><path d="M6.5 13.5V10h3v3.5"/></symbol>
  <symbol id="grid" viewBox="0 0 16 16"><rect x="2.5" y="2.5" width="4.5" height="4.5"/><rect x="9" y="2.5" width="4.5" height="4.5"/><rect x="2.5" y="9" width="4.5" height="4.5"/><rect x="9" y="9" width="4.5" height="4.5"/></symbol>
  <symbol id="swap" viewBox="0 0 16 16"><path d="M5 2.5 2.5 5 5 7.5M2.5 5H12M11 8.5l2.5 2.5L11 13.5M13.5 11H4"/></symbol>
  <symbol id="wallet" viewBox="0 0 16 16"><path d="M2.5 4.5h10v9h-10zM2.5 4.5l8-2v2"/><path d="M10 9h1"/></symbol>
  <symbol id="spark" viewBox="0 0 16 16"><path d="M7.5 2c.4 3.2 1.6 4.4 4.8 4.8-3.2.4-4.4 1.6-4.8 4.8C7.1 8.4 5.9 7.2 2.7 6.8 5.9 6.4 7.1 5.2 7.5 2z"/><path d="M12.5 10.5v3M11 12h3"/></symbol>
  <symbol id="star" viewBox="0 0 16 16"><path d="M8 2l1.8 3.8 4.2.5-3.1 2.9.8 4.1L8 11.3l-3.7 2 .8-4.1L2 6.3l4.2-.5z"/></symbol>
  <symbol id="share" viewBox="0 0 16 16"><path d="M8 10V2.5M5 5.5l3-3 3 3M3 9v4.5h10V9"/></symbol>
  <symbol id="play" viewBox="0 0 16 16"><path d="M5 3v10l8-5z" fill="currentColor" stroke="none"/></symbol>
  <symbol id="pause" viewBox="0 0 16 16"><path d="M4.5 3h2.5v10H4.5zM9 3h2.5v10H9z" fill="currentColor" stroke="none"/></symbol>
  <symbol id="plus" viewBox="0 0 16 16"><path d="M8 3v10M3 8h10"/></symbol>
  <symbol id="send" viewBox="0 0 16 16"><path d="M4.5 11.5l7-7M6 4.5h5.5V10"/></symbol>
  <symbol id="recv" viewBox="0 0 16 16"><path d="M8 2.5v10M4 8.5l4 4 4-4"/></symbol>
  <symbol id="users" viewBox="0 0 16 16"><rect x="3.5" y="2.5" width="4" height="4"/><path d="M1.5 13c.4-2.6 2-4 4-4s3.6 1.4 4 4"/><path d="M10 3h2.5v3.5H10M11.5 9.5c1.5.4 2.6 1.6 3 3.5"/></symbol>
  <symbol id="flask" viewBox="0 0 16 16"><path d="M6 2h4M6.5 2v4L3 13.5h10L9.5 6V2M4.5 10.5h7"/></symbol>
  <symbol id="shield" viewBox="0 0 16 16"><path d="M8 2l5 2v4c0 3-2.2 5.2-5 6-2.8-.8-5-3-5-6V4z"/></symbol>
  <symbol id="cols" viewBox="0 0 16 16"><rect x="2.5" y="3" width="4.5" height="10"/><rect x="9" y="3" width="4.5" height="10"/></symbol>
  <symbol id="copy" viewBox="0 0 16 16"><rect x="5.5" y="5.5" width="8" height="8"/><path d="M10.5 5.5v-3h-8v8h3"/></symbol>
  <symbol id="bolt" viewBox="0 0 16 16"><path d="M9 2 3.5 9H8l-1 5 5.5-7H8z"/></symbol>
  <symbol id="chat" viewBox="0 0 16 16"><path d="M2.5 3h11v8h-7l-4 3z"/></symbol>
  <symbol id="book" viewBox="0 0 16 16"><path d="M3 2.5h9.5v11H3zM5.5 2.5v11"/></symbol>
  <symbol id="clock" viewBox="0 0 16 16"><rect x="2.5" y="2.5" width="11" height="11"/><path d="M8 5v3.5l2 1.5"/></symbol>
  <symbol id="user" viewBox="0 0 16 16"><circle cx="8" cy="5.5" r="2.8"/><path d="M2.8 14c.6-3 2.7-4.6 5.2-4.6s4.6 1.6 5.2 4.6"/></symbol>
  <symbol id="lock" viewBox="0 0 16 16"><rect x="3" y="7" width="10" height="7" rx="2"/><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2"/></symbol>
  <symbol id="globe" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6"/><path d="M2 8h12M8 2c1.8 2 1.8 10 0 12M8 2c-1.8 2-1.8 10 0 12"/></symbol>
  <symbol id="help" viewBox="0 0 16 16"><circle cx="8" cy="8" r="6"/><path d="M6.3 6.2A1.8 1.8 0 0 1 9.8 6.6c0 1.3-1.8 1.5-1.8 2.6M8 11.3v.2"/></symbol>
  <symbol id="out" viewBox="0 0 16 16"><path d="M9.5 3H4v10h5.5M7 8h7M11.5 5.5 14 8l-2.5 2.5"/></symbol>
  <symbol id="gift" viewBox="0 0 16 16"><rect x="2.5" y="6" width="11" height="7.5" rx="1.5"/><path d="M2 6h12M8 6v7.5M8 6C6.5 3 4 3.5 4.6 5.2 5 6 8 6 8 6zM8 6c1.5-3 4-2.5 3.4-.8C11 6 8 6 8 6z"/></symbol>
  <symbol id="key" viewBox="0 0 16 16"><circle cx="5.5" cy="10.5" r="3"/><path d="M7.7 8.3 13.5 2.5M11 5l1.5 1.5"/></symbol>
  <symbol id="face" viewBox="0 0 16 16"><path d="M2.5 5.5v-2a1 1 0 0 1 1-1h2M10.5 2.5h2a1 1 0 0 1 1 1v2M13.5 10.5v2a1 1 0 0 1-1 1h-2M5.5 13.5h-2a1 1 0 0 1-1-1v-2M6 6.5v1M10 6.5v1M8 7v2.2H7.4M6.2 11c1 .8 2.6.8 3.6 0"/></symbol>
  <symbol id="status" viewBox="0 0 58 12"><g stroke="none"><rect x="0" y="8" width="3" height="4"/><rect x="4.5" y="6" width="3" height="6"/><rect x="9" y="3.5" width="3" height="8.5"/><rect x="13.5" y="1" width="3" height="11"/><path d="M27 3.2a10 10 0 0 1 7.4 2.9l-1.3 1.3A8.2 8.2 0 0 0 27 5a8.2 8.2 0 0 0-6.1 2.4L19.6 6.1A10 10 0 0 1 27 3.2zm0 3.7a5.6 5.6 0 0 1 4.3 1.8l-1.3 1.3a3.9 3.9 0 0 0-6 0l-1.3-1.3A5.6 5.6 0 0 1 27 6.9zm0 3.6 1.4 1.4L27 13.3l-1.4-1.4z"/><rect x="38.5" y="1.5" width="17" height="9" rx="2.6" fill="none" stroke="currentColor" stroke-width="1" opacity=".5"/><rect x="40" y="3" width="12" height="6" rx="1.5"/></g></symbol>`;

export function Sprite() {
  return (
    <svg width="0" height="0" style={{ position: "absolute" }} aria-hidden="true" dangerouslySetInnerHTML={{ __html: SPRITE }} />
  );
}
