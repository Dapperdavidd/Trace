/**
 * Example data from the TRACE design (design/trace-redesign-v11.html).
 * Every value here is fictional. Screens show it only when the live API has nothing yet,
 * and always with an "Example" label.
 */
export interface ExampleCoin { n: string; p: number; ch: number; vol: string; mc: string; liq: string; liqN: number; mcN: number; hold: string; why: string; sm: number; risk: "Low" | "Medium" | "High"; rn: number; seed: number; vx: string; early?: number }
export interface ExampleWallet { a: string; av: string; ret: number; win: number; trades: number; hold: string; d7: number; d30: number; d90: number; style: string[]; copiers: string; seed: number; isNew?: number }
export interface ExampleEvent { t: number; w: string; amt: string; sell?: number }
export interface ExampleData {
  coins: Record<string, ExampleCoin>;
  order: string[];
  wallets: Record<string, ExampleWallet>;
  wOrder: string[];
  events: Record<string, ExampleEvent[]>;
  holdings: [string, string, string, number, number, number][];
  activity: string[][];
  risk: Record<string, { s: string; c: [string, string, string][] }>;
}

export const D: ExampleData = {
  coins:{
    NEURA:{n:"Neura Protocol",p:0.0084,ch:42.7,vol:"$12.4M",mc:"$8.4M",liq:"$3.1M",liqN:3.1,mcN:8.4,hold:"4,382",why:"3 smart wallets bought",sm:3,risk:"Medium",rn:3,seed:3,vx:"4.8x"},
    SOLX:{n:"Solx",p:0.132,ch:28.4,vol:"$8.1M",mc:"$12.1M",liq:"$4.2M",liqN:4.2,mcN:12.1,hold:"9,120",why:"Volume 2x normal",sm:2,risk:"Low",rn:1,seed:23,vx:"2.0x"},
    FROG:{n:"Frog",p:0.0031,ch:19.1,vol:"$6.3M",mc:"$3.0M",liq:"$1.2M",liqN:1.2,mcN:3.0,hold:"2,947",why:"One wallet holds 22%",sm:1,risk:"Medium",rn:3,seed:5,vx:"3.1x"},
    NOVA:{n:"Nova",p:0.21,ch:15.8,vol:"$4.9M",mc:"$21.0M",liq:"$2.6M",liqN:2.6,mcN:21,hold:"11,402",why:"Trending on social",sm:1,risk:"Low",rn:2,seed:41,vx:"1.6x"},
    ORBIT:{n:"Orbit",p:0.045,ch:12.3,vol:"$3.1M",mc:"$4.5M",liq:"$0.4M",liqN:0.4,mcN:4.5,hold:"612",why:"Same 4 wallets trading",sm:0,risk:"High",rn:5,seed:17,vx:"5.2x"},
    ZETA:{n:"Zeta",p:0.087,ch:11.9,vol:"$2.7M",mc:"$6.2M",liq:"$1.9M",liqN:1.9,mcN:6.2,hold:"3,310",why:"Large buyer entered",sm:1,risk:"Low",rn:2,seed:38,vx:"1.4x"},
    BETA:{n:"Beta",p:0.018,ch:4.1,vol:"$1.1M",mc:"$2.2M",liq:"$0.9M",liqN:0.9,mcN:2.2,hold:"1,904",why:"5 smart wallets, price flat",sm:5,risk:"Low",rn:2,seed:61,vx:"1.9x",early:1},
    MIND:{n:"Mind",p:0.0062,ch:2.3,vol:"$0.6M",mc:"$1.4M",liq:"$0.5M",liqN:0.5,mcN:1.4,hold:"1,120",why:"3 smart wallets, price flat",sm:3,risk:"Medium",rn:3,seed:72,vx:"1.5x",early:1},
    LUNA:{n:"Luna",p:0.0047,ch:1.8,vol:"$0.4M",mc:"$0.9M",liq:"$0.2M",liqN:0.2,mcN:0.9,hold:"840",why:"4 smart wallets, thin liquidity",sm:4,risk:"High",rn:4,seed:83,vx:"1.3x",early:1}
  },
  order:["NEURA","SOLX","FROG","NOVA","ORBIT","ZETA","BETA","MIND","LUNA"],
  wallets:{
    "0xSun":{a:"7a2F…9kP",av:"0x",ret:1324,win:78,trades:432,hold:"2.8 days",d7:12.1,d30:48.6,d90:131,style:["Early buyer","Holds 1 to 7 days","Small caps"],copiers:"1,840",seed:31},
    "CryptoNomad":{a:"3mB1…2dQ",av:"CN",ret:892,win:72,trades:288,hold:"6.1 days",d7:5.4,d30:22.3,d90:61,style:["Swing","Large caps"],copiers:"2,310",seed:44},
    "DeFiWhale":{a:"9fK4…1pL",av:"DW",ret:684,win:66,trades:351,hold:"8.4 days",d7:-1.2,d30:-3.1,d90:38,style:["Mid caps","Holds 3 to 14 days"],copiers:"3,120",seed:52},
    "AlphaHunter":{a:"2xV9…7kD",av:"AH",ret:512,win:71,trades:190,hold:"1.2 days",d7:9.8,d30:31.7,d90:74,style:["Memecoins","Early entry"],copiers:"960",seed:9},
    "Nyx":{a:"5qR2…8eW",av:"NX",ret:211,win:69,trades:74,hold:"3.0 days",d7:4.4,d30:18.9,d90:44,style:["New","Small caps"],copiers:"120",seed:66,isNew:1}
  },
  wOrder:["0xSun","CryptoNomad","DeFiWhale","AlphaHunter","Nyx"],
  events:{
    NEURA:[{t:24,w:"0xSun",amt:"$18K"},{t:26,w:"CryptoNomad",amt:"$14K"},{t:31,w:"AlphaHunter",amt:"$9K"},{t:176,w:"0xSun",amt:"$127K"}],
    SOLX:[{t:10,w:"0xSun",amt:"$89K"},{t:162,w:"CryptoNomad",amt:"$8.9K"}],
    FROG:[{t:70,w:"AlphaHunter",amt:"$22K"},{t:138,w:"DeFiWhale",amt:"$15K",sell:1}],
    NOVA:[{t:46,w:"0xSun",amt:"$73K"},{t:130,w:"AlphaHunter",amt:"$6.2K"}],
    ORBIT:[{t:40,w:"Nyx",amt:"$2K"}],
    ZETA:[{t:96,w:"CryptoNomad",amt:"$31K"}],
    BETA:[{t:60,w:"0xSun",amt:"$12K"},{t:88,w:"AlphaHunter",amt:"$7K"},{t:120,w:"CryptoNomad",amt:"$9K"},{t:150,w:"Nyx",amt:"$3K"}],
    MIND:[{t:72,w:"0xSun",amt:"$6K"},{t:140,w:"Nyx",amt:"$2K"}],
    LUNA:[{t:55,w:"AlphaHunter",amt:"$4K"},{t:132,w:"Nyx",amt:"$3K"}]
  },
  holdings:[["SOL","Solana","8.20 SOL",1166.86,6.8,41.1],["USDC","USD Coin","700.00 USDC",700,0,24.6],["NEURA","NEURA","45,231 NEURA",379.94,42.7,13.4],["ETH","Ethereum","0.12 ETH",308.25,3.1,10.9],["FROG","FROG","92,000 FROG",285.2,19.1,10.0]],
  activity:[["0xSun","bought","NEURA","$127K","11:56","Copied for you · $100"],["CryptoNomad","bought","SOLX","$8.9K","11:42",""],["DeFiWhale","sold","FROG","$15K","11:18","Copied for you"],["AlphaHunter","bought","NOVA","$6.2K","11:10",""]],
  risk:{
    NEURA:{s:"The contract looks safe, but a few wallets hold a lot of the supply. Expect sharp swings.",c:[["Liquidity","$3.1M, locked 6 months","Good"],["Can you sell?","A test sell went through","Pass"],["Holder concentration","Top 10 wallets hold 38%","Watch"],["Contract","Verified, no mint function","Good"],["Suspicious trading","No wash trading found","None"]]},
    ORBIT:{s:"Most of the volume is 4 wallets trading with each other. The price move is not real demand.",c:[["Liquidity","$0.4M, not locked","Watch"],["Can you sell?","A test sell went through","Pass"],["Holder concentration","Top 10 wallets hold 71%","Flag"],["Contract","Not verified","Flag"],["Suspicious trading","Wash loop between 4 wallets","Flag"]]},
    FROG:{s:"One wallet holds 22% of the supply. If it sells, the price drops hard.",c:[["Liquidity","$1.2M, locked 3 months","Good"],["Can you sell?","A test sell went through","Pass"],["Holder concentration","One wallet holds 22%","Watch"],["Contract","Verified, no mint function","Good"],["Suspicious trading","No wash trading found","None"]]},
    LUNA:{s:"Smart wallets are buying, but liquidity is thin. A $5K sell moves the price 9%.",c:[["Liquidity","$0.2M, not locked","Flag"],["Can you sell?","A test sell went through","Pass"],["Holder concentration","Top 10 wallets hold 44%","Watch"],["Contract","Verified","Good"],["Suspicious trading","No wash trading found","None"]]}
  }
};
export const RANGE: Record<string,{d:string;seed:number}> = {"1H":{d:"+$18.40 · +0.6% 1h",seed:5},"1D":{d:"+$313.33 · +12.4% 24h",seed:17},"1W":{d:"+$402.10 · +16.5% 1w",seed:21},"1M":{d:"+$611.92 · +27.5% 1m",seed:33},"1Y":{d:"+$1,904.11 · +203% 1y",seed:47}};
export const HOLD_SOL: Record<number,number> = {7:2.1,30:9.2,90:18.4};
export const SOLP = 142.30;
export const STORY = [
  {short:"Meet 0xSun",lens:"w0",dur:3.4,cap:"This is <b>0xSun</b>, a wallet that has been right on 78% of its trades. trace keeps an eye on wallets like this for you.",capS:"This is <b>0xSun</b>. It has been right on 78% of its trades."},
  {short:"126.5 SOL on the move",lens:"p0",dur:3.2,paths:[["w0","swap",3]],cap:"At <b>09:24</b> it sends <b>126.5 SOL</b>, about $18,000, into a swap.",capS:"At 09:24 it sends <b>126.5 SOL</b>, about $18,000, into a swap."},
  {short:"Swapped for NEURA",lens:"p0",dur:3.2,swap:1,paths:[["swap","NEURA",2.5]],cap:"The swap turns it into <b>3.05M NEURA</b>. Almost nobody is talking about NEURA yet.",capS:"The swap turns it into <b>3.05M NEURA</b>."},
  {short:"Others follow",lens:"p0",dur:3.6,paths:[["w1","NEURA",3],["w2","NEURA",4]],cap:"Within 7 minutes, <b>two more wallets</b> with strong records buy NEURA too.",capS:"Within 7 minutes, <b>two more proven wallets</b> buy too."},
  {short:"NEURA +42.7%",lens:"NEURA",dur:4,rise:"NEURA",cap:"51 minutes later NEURA jumps <b>42.7%</b>. trace flagged it at 09:24, before the move.",capS:"51 minutes later NEURA jumps <b>42.7%</b>. trace saw it first."}
];

