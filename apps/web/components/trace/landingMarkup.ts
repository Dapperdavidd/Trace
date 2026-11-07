/* Landing markup ported from design/trace-redesign-v11.html (section #landing), with links pointed at real routes. */
export const LANDING_HTML = `<section id="landing" class="L" aria-label="Landing page">
  <header class="Lnav"><div class="wrap">
    <span class="logo"><svg viewBox="0 0 24 24"><use href="#lens"/></svg>trace</span>
    <nav><a href="#start">Get started</a><a href="#inside">Product</a><a href="#setup">Set up</a><a href="#faq">Questions</a></nav>
    <div class="bset"><a class="blink" href="/welcome/" style="height:38px">Log in</a><a class="bfill sm" href="/app/today/">Open trace</a></div>
  </div></header>

  <div class="Lfull">
    <div class="Lfull-3d" id="heroField"></div>
    <div class="Lfull-t"><div class="wrap">
      <span class="pill"><i></i>On-chain research, made simple</span>
      <h1>See who's buying <span>before you buy.</span></h1>
      <p class="Llede">trace reads every block on the chain and tells you, in plain words, who is buying a coin, whether it looks safe, and what would have happened if you had copied them.</p>
      <div class="bset"><a class="bfill" href="/welcome/">Open trace</a><a class="bout" href="#inside">See what it does</a></div>
    </div></div>
    <div class="Lfull-cap"><i class="dot live"></i><span id="peekTxt">Reading block #18,240,214 · 241 trades</span></div>
    <div class="Lfull-hint">Move your cursor over the blocks · click one to open it</div>
  </div>

      <section class="Ow" id="start" aria-label="Get started">
    <div class="Ow-pin">
      <div class="wrap Ow-in">
        <div class="Ow-h"><h2><span class="hl">Trace</span><span class="hl g">your way.</span></h2><p>Start in your browser or on your iPhone. Your wallet and everything you follow come with you.</p></div>
        <div class="Ow-card" id="owCard">
          <div class="Ow-dash" id="owDash" role="tablist" aria-label="Ways to use trace"><button class="on" aria-label="Web app"></button><button aria-label="iPhone"></button><button aria-label="Developers"></button></div>
          <article class="Ow-s on" data-i="0">
            <div class="Ow-t"><span class="Ow-n">001</span><span class="Ow-k">Web app</span><h3>The full trace, on a big screen.</h3><p>Research, copy and swap from your browser. Nothing to install.</p><a class="Ow-cta" href="/app/today/"><span>Open trace</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
            <div class="Ow-v"><div class="mbrowser" aria-hidden="true"><div class="mb-bar"><i></i><i></i><i></i><span>app.trace.xyz</span></div><div class="mb-body"><div class="mb-side"><b></b><span class="on"></span><span></span><span></span><span></span><span></span></div><div class="mb-main"><div class="mb-bal"><small>Portfolio</small><b>$2,840.25</b><em>+12.4% today</em></div><div class="mb-chart" id="mbChart"></div><div class="mb-rows"><div><span class="tk on">N</span>NEURA<em>+42.7%</em></div><div><span class="tk">S</span>SOLX<em>+28.4%</em></div><div><span class="tk">B</span>BETA<em>+4.1%</em></div></div></div></div></div></div>
          </article>
          <article class="Ow-s" data-i="1">
            <div class="Ow-t"><span class="Ow-n">002</span><span class="Ow-k">iPhone</span><h3>In your pocket, with alerts.</h3><p>The trace phone app is in private beta. You hear the moment a wallet you follow buys, and a passkey guards your account.</p><a class="Ow-cta" href="/welcome/"><span>Get early access</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
            <div class="Ow-v Ow-ph"><div class="ph-mini" id="phMini"><div class="island"></div><div class="app mob"><div class="msb"><span>9:41</span><svg><use href="#status"/></svg></div><div class="vp"></div></div></div></div>
          </article>
          <article class="Ow-s" data-i="2">
            <div class="Ow-t"><span class="Ow-n">003</span><span class="Ow-k">Developers</span><h3>The same signals, as an API.</h3><p>Smart wallet buys as webhooks, and the safety check for any coin. Build them into your own app.</p><a class="Ow-cta" href="https://trace-api-dapper.onrender.com/v1/openapi.json" target="_blank" rel="noreferrer"><span>API docs</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
            <div class="Ow-v"><pre class="code" aria-label="Example webhook"><span class="c3">POST</span> /webhooks/trace
{
  <span class="c2">"type"</span>: <span class="c1">"smart_wallet.buy"</span>,
  <span class="c2">"wallet"</span>: <span class="c1">"0xSun"</span>,
  <span class="c2">"coin"</span>: <span class="c1">"NEURA"</span>,
  <span class="c2">"usd"</span>: 18000,
  <span class="c2">"win_rate"</span>: 0.78,
  <span class="c2">"block"</span>: 18240113
}</pre></div>
          </article>
        </div>
      </div>
    </div>
  </section>

  <section class="wrap Sk" id="inside" aria-label="What's inside">
    <div class="Sk-h"><h2><span class="hl">What's</span><span class="hl g">inside.</span></h2><p>Six tools that do the reading for you. Scroll through them, or tap one to try it.</p></div>
    <article class="sk" style="--i:0">
      <h3>Daily briefing</h3>
      <div class="sk-g">
        <div class="sk-l"><div class="sk-tag"><span>01</span>What's inside</div><p>A short read every morning: what moved, who bought, and what to avoid. Or listen to it in two minutes.</p><div class="sk-ht"><span>Every morning</span><span>2-minute listen</span><span>Pick your time</span></div><a class="Ow-cta sm" href="/app/briefing/"><span>Try it</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
        <div class="sk-v"><div class="sv"><span class="sv-k">Saturday 7 November · 08:00</span><p class="sv-q">A quiet morning for majors. Smart money is rotating into two small caps, and one coin you watch failed a safety check.</p><div class="player on"><span class="pb"><svg class="i"><use href="#play"/></svg></span><div class="wave" id="btWave"></div><span class="f3" style="font-size:12px">2:04</span></div><div class="sv-rows"><div><b>3 new opportunities</b><span>NEURA, SOLX, BETA</span></div><div><b>2 wallet moves</b><span>0xSun, CryptoNomad</span></div><div><b>1 risk alert</b><span>PIXEL liquidity −61%</span></div></div></div></div>
      </div>
    </article>
    <article class="sk" style="--i:1">
      <h3>Coins with a reason</h3>
      <div class="sk-g">
        <div class="sk-l"><div class="sk-tag"><span>02</span>What's inside</div><p>Every coin on the list says why it is moving, so you never buy just because a number is green.</p><div class="sk-ht"><span>Live</span><span>Why it moves</span><span>Risk label</span></div><a class="Ow-cta sm" href="/app/discover/"><span>Try it</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
        <div class="sk-v"><div class="sv"><div class="sv-search"><svg class="i"><use href="#srch"/></svg>Search coins or reasons</div><div class="cwr" id="cwr"></div></div></div>
      </div>
    </article>
    <article class="sk" style="--i:2">
      <h3>Safety check</h3>
      <div class="sk-g">
        <div class="sk-l"><div class="sk-tag"><span>03</span>What's inside</div><p>Five tests on every coin before you buy. Each one says what was checked, in plain words.</p><div class="sk-ht"><span>Can you sell</span><span>Who holds it</span><span>Is the trading real</span></div><a class="Ow-cta sm" href="/app/token/?sym=NEURA&amp;tab=rk"><span>Try it</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
        <div class="sk-v"><div class="sv"><div class="sfm"><b>NEURA · Medium risk</b><span>3 of 5</span></div><div class="meter"><i class="on"></i><i class="on"></i><i class="on"></i><i></i><i></i></div><div class="sv-rows"><div><b>Can you sell?</b><span class="tag g">Passed</span></div><div><b>Liquidity locked 6 months</b><span class="tag g">Looks good</span></div><div><b>Top 10 wallets hold 38%</b><span class="tag">Keep an eye on it</span></div><div><b>Contract verified</b><span class="tag g">Looks good</span></div></div></div></div>
      </div>
    </article>
    <article class="sk" style="--i:3">
      <h3>Pretend money</h3>
      <div class="sk-g">
        <div class="sk-l"><div class="sk-tag"><span>04</span>What's inside</div><p>Replay any wallet as if you had copied it. Fees and trades you would have missed are counted. Nothing is spent.</p><div class="sk-ht"><span>7, 30 or 90 days</span><span>Fees counted</span><span>Nothing spent</span></div><a class="Ow-cta sm" href="/app/shadow/"><span>Try it</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
        <div class="sk-v"><div class="sv"><span class="sv-k">$100 copying 0xSun · 30 days</span><div class="pmv"><b>$148.60</b><span>Holding SOL would be $109.20</span></div><div id="pmChart"></div><div class="legend"><span><i></i>Copying 0xSun</span><span><i class="d"></i>Holding SOL</span></div></div></div>
      </div>
    </article>
    <article class="sk" style="--i:4">
      <h3>Copy with limits</h3>
      <div class="sk-g">
        <div class="sk-l"><div class="sk-tag"><span>05</span>What's inside</div><p>Copy a wallet you trust with safety limits switched on from the very first trade. The limits live in a NEAR mandate that only your wallet can sign, pause or revoke.</p><div class="sk-ht"><span>Amount per trade</span><span>Skip risky coins</span><span>Signed on NEAR</span></div><a class="Ow-cta sm" href="/app/copy/"><span>Try it</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
        <div class="sk-v"><div class="sv"><span class="sv-k">Copy 0xSun · right 78% of the time</span><div class="amtb"><span>$50</span><span class="on">$100</span><span>$250</span><span>$500</span></div><div class="cpl"><div><span>Only coins with good liquidity</span><i class="sw on"></i></div><div><span>Skip high-risk coins</span><i class="sw on"></i></div><div><span>Max 5% of portfolio per trade</span><i class="sw on"></i></div><div><span>Expires in 7 days unless you renew</span><i class="sw on"></i></div></div></div></div>
      </div>
    </article>
    <article class="sk" style="--i:5">
      <h3>Ask trace</h3>
      <div class="sk-g">
        <div class="sk-l"><div class="sk-tag"><span>06</span>What's inside</div><p>Ask anything in plain words. Answers come back as coins and wallets you can tap.</p><div class="sk-ht"><span>Plain words</span><span>Answers you can tap</span></div><a class="Ow-cta sm" href="/app/ask/"><span>Try it</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
        <div class="sk-v"><div class="sv"><span class="qb2" style="justify-self:end">Which coins are good wallets buying that haven't moved yet?</span><div class="askans"><span class="who"><svg class="i"><use href="#spark"/></svg>trace</span><p class="sv-q" style="font-size:14.5px">3 coins match. Each has 3 or more proven wallets buying, and the price is up less than 5%.</p><div class="chips"><span><b>BETA</b> 5 smart wallets</span><span><b>MIND</b> 3 smart wallets</span><span><b>LUNA</b> higher risk</span></div></div></div></div>
      </div>
    </article>
  </section>

  <section class="Su" id="setup" aria-label="Set up in 5 steps">
    <div class="Su-pin">
      <div class="wrap Su-h"><h2><span class="hl">Set up in</span><span class="hl g">5 steps.</span></h2><p>About three minutes, start to finish.</p></div>
      <div class="Su-track" id="suTrack">
        <div class="su" style="--o:0"><span class="su-tag">Step 1</span><h3>Account <i>01</i></h3><div><b>Create a passkey</b><span>Face, fingerprint or device PIN</span></div><div><b>No password</b><span>Nothing to leak or forget</span></div><div><b>Connect NEAR</b><span>Your wallet keeps its keys</span></div></div>
        <div class="su" style="--o:1"><span class="su-tag">Step 2</span><h3>Follow <i>02</i></h3><div><b>Pick smart wallets</b><span>Ranked by 30-day return</span></div><div><b>Star coins</b><span>Your watchlist</span></div><div><b>Turn on alerts</b><span>Within a minute of a buy</span></div></div>
        <div class="su" style="--o:2"><span class="su-tag">Step 3</span><h3>Test <i>03</i></h3><div><b>Pretend money</b><span>Try any wallet first</span></div><div><b>7, 30 or 90 days</b><span>Your amount, your period</span></div><div><b>Fees counted</b><span>No surprises later</span></div></div>
        <div class="su" style="--o:3"><span class="su-tag">Step 4</span><h3>Copy <i>04</i></h3><div><b>Amount per trade</b><span>From $50</span></div><div><b>Guardrails</b><span>Skip risky coins</span></div><div><b>Sign on NEAR</b><span>Pause or revoke any time</span></div></div>
        <div class="su" style="--o:4"><span class="su-tag">Step 5</span><h3>Make it yours <i>05</i></h3><div><b>Briefing time</b><span>7:00, 8:00 or noon</span></div><div><b>Currency</b><span>USD, NGN, GBP, EUR, THB</span></div><div><b>Spending limit</b><span>A cap for each day</span></div></div>
        <div class="su su-end"><h3>That's it.</h3><p>You're reading the chain like a pro.</p><a class="Ow-cta" href="/app/today/"><span>Open trace</span><i><svg class="i"><use href="#arr"/></svg></i></a></div>
      </div>
    </div>
  </section>

<section class="wrap Hs" id="faq">
    <div class="Hs-l"><h2>Questions</h2><p>Plain answers.</p></div>
    <div class="Hs-r"><div class="Hpanel Hfaq">
      <details open><summary>Do I need to understand crypto?</summary><p>No. Everything is written in plain words. If you can read “3 wallets that are usually right just bought this”, you can use trace.</p></details>
      <details><summary>Does trace hold my money?</summary><p>No. trace never holds keys or funds. Your own NEAR wallet signs every mandate, and nobody at trace can move your money.</p></details>
      <details><summary>What is a smart wallet?</summary><p>A wallet with a long record of good trades. trace only calls a wallet smart after 50 or more trades, and always shows you its win rate.</p></details>
      <details><summary>Is copying a wallet safe?</summary><p>Copying always carries risk. That is why you test with pretend money first, and why every copy runs under limits you sign on NEAR: an amount per trade, a daily cap and an expiry date.</p></details>
      <details><summary>Which blockchains does it read?</summary><p>Ethereum (Uniswap V2) and Solana (Raydium) today. NEAR is where your agent permissions live: every copy limit is a NEAR mandate you sign. Base and BNB Chain are next.</p></details>
    </div></div>
  </section>

  <section class="wrap Hcta" id="get">
    <div class="Hcta-h"><h2>Get trace</h2><p>Free on the web, iPhone and Android.</p></div>
    <div class="Hstats">
      <div><b>18.2<small>m</small></b><span>blocks read, last 30 days</span></div>
      <div><b>4,180</b><span>smart wallets tracked</span></div>
      <div><b>212<small>k</small></b><span>coins safety checked</span></div>
      <div><b>9<small>s</small></b><span>from trade to alert</span></div>
    </div>
    <div class="Hact">
      <form class="Hmail" id="hmail"><input id="hmailIn" type="email" placeholder="Your email for the download link" aria-label="Email" autocomplete="off"><button type="submit" aria-label="Send me the link"><svg class="i"><use href="#arr"/></svg></button></form>
      <a class="bout" href="/app/today/">Open web app</a><a class="bfill" href="/welcome/">Create an account</a>
    </div>
    <p class="Hnote" id="hmailNote" aria-live="polite"></p>
  </section>

  <footer class="wrap Hfoot">
    <div class="Hfoot-b"><span class="logo"><svg viewBox="0 0 24 24"><use href="#lens"/></svg>trace</span><p>See who's buying before you buy. Your keys stay with you.</p></div>
    <div><h4>Product</h4><a href="/app/today/">Web app</a><a href="#start">iPhone</a><a href="#start">Android</a><a href="#start">Pricing</a><a href="#setup">What's new</a></div>
    <div><h4>Learn</h4><a href="#setup">Guides</a><a href="#setup">Blocks explained</a><a href="#setup">Smart wallets</a><a href="#inside">Safety check</a></div>
    <div><h4>Developers</h4><a href="https://trace-api-dapper.onrender.com/v1/openapi.json" target="_blank" rel="noreferrer">API</a><a href="https://github.com/Dapperdavidd/Trace" target="_blank" rel="noreferrer">GitHub</a><a href="/app/near/">NEAR agent</a></div>
    <div><h4>Community</h4><a href="#get">X</a><a href="#get">Discord</a><a href="#get">Telegram</a></div>
    <div><h4>Company</h4><a href="#get">About</a><a href="#get">Careers</a><a href="/privacy/">Privacy</a><a href="/terms/">Terms</a></div>
    <div class="Hfoot-s"><span>© 2026 trace</span><span id="apiStatus"><i class="dot"></i>Checking the trace API</span><span>Numbers, wallets and coins on this page are examples.</span></div>
  </footer>
</section>`;
