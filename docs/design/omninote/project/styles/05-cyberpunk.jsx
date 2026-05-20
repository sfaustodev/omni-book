// Cyberpunk / Synthwave — neon, scanlines, magenta+cyan
const Cyberpunk = () => {
  const c = {
    bg: '#0a0014',
    panel: 'rgba(20,5,40,0.7)',
    border: '#ff00aa',
    border2: '#00f0ff',
    text: '#f0f0ff',
    dim: '#8888aa',
    magenta: '#ff2bd6',
    cyan: '#00f0ff',
    yellow: '#fff352',
    green: '#39ff88',
  };
  const mono = "'JetBrains Mono', 'Space Mono', monospace";

  const glow = (col) => `0 0 8px ${col}, 0 0 18px ${col}55`;

  const Item = ({ label, active, badge }) => (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 10, padding: '7px 12px',
      borderLeft: active ? `3px solid ${c.magenta}` : '3px solid transparent',
      background: active ? 'linear-gradient(90deg, rgba(255,43,214,0.15), transparent)' : 'transparent',
      color: active ? c.text : c.dim, fontFamily: mono, fontSize: 12, cursor: 'pointer',
      textShadow: active ? `0 0 6px ${c.magenta}88` : 'none',
    }}>
      <span style={{ color: active ? c.magenta : c.dim }}>{active ? '▶' : '·'}</span>
      <span style={{ flex: 1, letterSpacing: 0.3 }}>{label}</span>
      {badge && <span style={{ fontSize: 10, color: c.cyan, border: `1px solid ${c.cyan}`, padding: '0 4px', textShadow: `0 0 6px ${c.cyan}` }}>{badge}</span>}
    </div>
  );

  return (
    <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: mono, position: 'relative', overflow: 'hidden' }}>
      {/* Grid background */}
      <div style={{ position: 'absolute', inset: 0, backgroundImage: `linear-gradient(rgba(255,43,214,0.06) 1px, transparent 1px), linear-gradient(90deg, rgba(0,240,255,0.06) 1px, transparent 1px)`, backgroundSize: '40px 40px' }}></div>
      {/* Scanlines */}
      <div style={{ position: 'absolute', inset: 0, backgroundImage: `repeating-linear-gradient(0deg, transparent 0, transparent 2px, rgba(255,255,255,0.015) 2px, rgba(255,255,255,0.015) 4px)`, pointerEvents: 'none' }}></div>
      {/* Glow */}
      <div style={{ position: 'absolute', width: 600, height: 600, borderRadius: '50%', background: `radial-gradient(circle, ${c.magenta}33 0%, transparent 70%)`, top: -200, right: -100, filter: 'blur(40px)' }}></div>
      <div style={{ position: 'absolute', width: 500, height: 500, borderRadius: '50%', background: `radial-gradient(circle, ${c.cyan}22 0%, transparent 70%)`, bottom: -100, left: -100, filter: 'blur(40px)' }}></div>

      <div style={{ position: 'relative', display: 'flex', height: '100%' }}>
        {/* Sidebar */}
        <div style={{ width: 240, background: c.panel, backdropFilter: 'blur(20px)', borderRight: `1px solid ${c.magenta}66`, display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '14px 14px 10px', borderBottom: `1px solid ${c.magenta}44` }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <div style={{ width: 26, height: 26, border: `1px solid ${c.magenta}`, transform: 'rotate(45deg)', boxShadow: glow(c.magenta), display: 'grid', placeItems: 'center' }}>
                <span style={{ transform: 'rotate(-45deg)', color: c.magenta, fontSize: 11, fontWeight: 700 }}>O</span>
              </div>
              <div>
                <div style={{ fontSize: 13, fontWeight: 700, color: c.magenta, textShadow: glow(c.magenta), letterSpacing: 2 }}>OMNI//NOTE</div>
                <div style={{ fontSize: 9, color: c.cyan, letterSpacing: 2 }}>v.2.0.26 · NETRUNNER</div>
              </div>
            </div>
          </div>

          <div style={{ padding: '10px 12px', borderBottom: `1px solid ${c.magenta}44`, display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ color: c.cyan, fontSize: 12, textShadow: glow(c.cyan) }}>{'>'}</span>
            <span style={{ color: c.dim, fontSize: 11, letterSpacing: 1 }}>QUERY//_</span>
            <span style={{ width: 6, height: 12, background: c.cyan, boxShadow: glow(c.cyan), animation: 'blink 1s infinite' }}></span>
          </div>

          <div style={{ padding: '8px 4px', flex: 1 }}>
            <div style={{ fontSize: 9, color: c.dim, padding: '6px 12px', letterSpacing: 2 }}>// FILTERS</div>
            <Item label="ALL.FILES" active badge="42" />
            <Item label="DRAFT.LOG" badge="07" />
            <Item label="ARCHIVE.Z" />

            <div style={{ fontSize: 9, color: c.dim, padding: '14px 12px 6px', letterSpacing: 2 }}>// NODES</div>
            <Item label="motoboy.spec" />
            <Item label="ifood.api" />
            <Item label="focus.nfe" />
            <Item label="call.log" />
          </div>

          <button style={{
            margin: 12, padding: '10px', background: 'transparent',
            border: `1px solid ${c.magenta}`, color: c.magenta,
            fontFamily: mono, fontSize: 11, letterSpacing: 2, fontWeight: 700,
            cursor: 'pointer', textShadow: glow(c.magenta), boxShadow: `inset 0 0 20px ${c.magenta}22`,
          }}>+ JACK_IN</button>

          <style>{`@keyframes blink { 50% { opacity: 0; } }`}</style>
        </div>

        {/* Editor */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
          <div style={{ padding: '10px 24px', borderBottom: `1px solid ${c.cyan}44`, display: 'flex', alignItems: 'center', gap: 14 }}>
            <span style={{ color: c.cyan, fontSize: 11, textShadow: glow(c.cyan), letterSpacing: 2 }}>NODE://motoboy.spec</span>
            <span style={{ marginLeft: 'auto', fontSize: 10, color: c.dim, letterSpacing: 2 }}>ENCRYPTED · LIVE · 23:59:14</span>
          </div>

          <div style={{ flex: 1, padding: '32px 48px', overflow: 'hidden' }}>
            <div style={{ fontSize: 11, color: c.cyan, letterSpacing: 3, marginBottom: 8, textShadow: glow(c.cyan) }}>// SPEC.001</div>
            <div style={{ fontSize: 44, fontWeight: 800, letterSpacing: 4, lineHeight: 1, marginBottom: 12, color: c.magenta, textShadow: `0 0 20px ${c.magenta}, 0 0 40px ${c.magenta}66` }}>
              MOTOBOYS_MODULE
            </div>
            <div style={{ fontSize: 13, color: c.dim, marginBottom: 28, letterSpacing: 1, maxWidth: 560 }}>
              {'>>'} iFood linking subroutine + Flutter mobile clients. Field deployment pending sec audit.
            </div>

            <div style={{ display: 'flex', gap: 8, marginBottom: 28 }}>
              {[['cfo-pocket', c.cyan], ['feat/motoboys', c.magenta], ['v1.0', c.yellow], ['LIVE', c.green]].map(([l, col]) => (
                <span key={l} style={{ padding: '3px 10px', border: `1px solid ${col}`, color: col, fontSize: 10, letterSpacing: 2, textShadow: `0 0 6px ${col}88` }}>{l}</span>
              ))}
            </div>

            <div style={{ border: `1px solid ${c.cyan}66`, padding: '18px 22px', marginBottom: 16, background: 'rgba(0,240,255,0.04)', position: 'relative' }}>
              <div style={{ position: 'absolute', top: -1, left: 14, padding: '0 8px', background: c.bg, color: c.cyan, fontSize: 10, letterSpacing: 2, textShadow: glow(c.cyan) }}>[ § 1 — GOAL ]</div>
              <div style={{ fontSize: 13, lineHeight: 1.7, color: c.text, marginTop: 4 }}>
                Deliver_op_flow:: motoboy <span style={{ color: c.magenta }}>{'>'}</span> Flutter.app <span style={{ color: c.magenta }}>{'>'}</span> register(order#, fee_tier) <span style={{ color: c.magenta }}>{'>'}</span> backend.persist <span style={{ color: c.magenta }}>{'>'}</span> link(iFood)? <span style={{ color: c.magenta }}>{'>'}</span> daily.checklist.auto
              </div>
            </div>

            <div style={{ border: `1px solid ${c.magenta}66`, padding: '18px 22px', background: 'rgba(255,43,214,0.04)', position: 'relative' }}>
              <div style={{ position: 'absolute', top: -1, left: 14, padding: '0 8px', background: c.bg, color: c.magenta, fontSize: 10, letterSpacing: 2, textShadow: glow(c.magenta) }}>[ § 2 — PREMISES ]</div>
              <div style={{ marginTop: 4 }}>
                {[
                  ['ICE_LVL_3', 'iFood integration ~95% on focusNFe branch'],
                  ['FAILSAFE', 'system continues if iFood goes dark'],
                  ['ISOLATE', 'Flutter app — dedicated motoboy login flow'],
                  ['NAMESPACE', 'all endpoints under /api/v1/motoboy/*'],
                ].map(([k, v]) => (
                  <div key={k} style={{ display: 'flex', gap: 12, padding: '5px 0', fontSize: 12 }}>
                    <span style={{ color: c.yellow, width: 90, letterSpacing: 1.5, textShadow: `0 0 6px ${c.yellow}66` }}>{k}</span>
                    <span style={{ color: c.text }}>{v}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
window.Cyberpunk = Cyberpunk;
