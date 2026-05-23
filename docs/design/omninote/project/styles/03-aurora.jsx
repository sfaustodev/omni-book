// Glassmorphism / Aurora — blur, gradients, frosted glass
const Aurora = () => {
  const c = {
    text: '#f0f2ff',
    dim: 'rgba(240,242,255,0.55)',
    dimmer: 'rgba(240,242,255,0.35)',
    glass: 'rgba(255,255,255,0.06)',
    glassStrong: 'rgba(255,255,255,0.10)',
    border: 'rgba(255,255,255,0.10)',
    pink: '#ff6ec7',
    blue: '#6ec1ff',
    green: '#7affd0',
  };
  const sans = "'Geist', 'Inter', -apple-system, sans-serif";

  const Item = ({ icon, label, active, count }) => (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 12, padding: '9px 12px', borderRadius: 10,
      background: active ? c.glassStrong : 'transparent',
      border: active ? `1px solid ${c.border}` : '1px solid transparent',
      color: active ? c.text : c.dim, fontSize: 13, cursor: 'pointer',
      backdropFilter: active ? 'blur(20px)' : 'none',
    }}>
      <span style={{ width: 14, color: active ? c.pink : c.dimmer }}>{icon}</span>
      <span style={{ flex: 1 }}>{label}</span>
      {count && <span style={{ fontSize: 11, color: c.dimmer }}>{count}</span>}
    </div>
  );

  const Chip = ({ children, color = c.blue }) => (
    <span style={{
      display: 'inline-flex', alignItems: 'center', gap: 6,
      padding: '4px 10px', borderRadius: 999,
      background: c.glass, border: `1px solid ${c.border}`,
      color, fontSize: 11, fontWeight: 500,
    }}>
      <span style={{ width: 5, height: 5, borderRadius: '50%', background: color, boxShadow: `0 0 8px ${color}` }}></span>
      {children}
    </span>
  );

  return (
    <div style={{ width: '100%', height: '100%', background: '#0a0612', color: c.text, fontFamily: sans, position: 'relative', overflow: 'hidden' }}>
      {/* Aurora background */}
      <div style={{ position: 'absolute', inset: 0, overflow: 'hidden' }}>
        <div style={{ position: 'absolute', width: 800, height: 800, borderRadius: '50%', background: 'radial-gradient(circle, #ff6ec7 0%, transparent 60%)', top: -200, left: -200, filter: 'blur(80px)', opacity: 0.5 }}></div>
        <div style={{ position: 'absolute', width: 700, height: 700, borderRadius: '50%', background: 'radial-gradient(circle, #6ec1ff 0%, transparent 60%)', bottom: -150, right: -100, filter: 'blur(80px)', opacity: 0.45 }}></div>
        <div style={{ position: 'absolute', width: 500, height: 500, borderRadius: '50%', background: 'radial-gradient(circle, #7affd0 0%, transparent 60%)', top: '40%', left: '40%', filter: 'blur(90px)', opacity: 0.25 }}></div>
        <div style={{ position: 'absolute', inset: 0, backgroundImage: `radial-gradient(circle at 1px 1px, rgba(255,255,255,0.06) 1px, transparent 0)`, backgroundSize: '24px 24px' }}></div>
      </div>

      <div style={{ position: 'relative', display: 'flex', height: '100%', padding: 16, gap: 16 }}>
        {/* Sidebar */}
        <div style={{ width: 250, background: c.glass, backdropFilter: 'blur(40px)', WebkitBackdropFilter: 'blur(40px)', border: `1px solid ${c.border}`, borderRadius: 20, padding: 14, display: 'flex', flexDirection: 'column' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '4px 6px 16px' }}>
            <div style={{ width: 30, height: 30, borderRadius: 9, background: 'linear-gradient(135deg, #ff6ec7, #6ec1ff)', display: 'grid', placeItems: 'center', color: '#0a0612', fontWeight: 700 }}>O</div>
            <div style={{ fontSize: 14, fontWeight: 600, letterSpacing: -0.2 }}>OmniNote</div>
            <div style={{ marginLeft: 'auto', width: 6, height: 6, borderRadius: '50%', background: c.green, boxShadow: `0 0 10px ${c.green}` }}></div>
          </div>

          <div style={{ background: c.glass, border: `1px solid ${c.border}`, borderRadius: 12, padding: '8px 12px', display: 'flex', alignItems: 'center', gap: 8, marginBottom: 14 }}>
            <span style={{ color: c.dimmer, fontSize: 13 }}>⌕</span>
            <span style={{ color: c.dim, fontSize: 12 }}>Search</span>
            <span style={{ marginLeft: 'auto', fontSize: 10, color: c.dimmer }}>⌘K</span>
          </div>

          <div style={{ fontSize: 10, color: c.dimmer, padding: '6px 10px', letterSpacing: 1.5 }}>VIEWS</div>
          <Item icon="◈" label="All notes" active count="42" />
          <Item icon="◊" label="Recent" />
          <Item icon="✦" label="Starred" count="3" />

          <div style={{ fontSize: 10, color: c.dimmer, padding: '14px 10px 6px', letterSpacing: 1.5 }}>SPACES</div>
          <Item icon="◐" label="Specs" count="12" />
          <Item icon="◑" label="Meetings" />
          <Item icon="◒" label="Daily logs" />

          <button style={{ marginTop: 'auto', background: 'linear-gradient(135deg, rgba(255,110,199,0.3), rgba(110,193,255,0.3))', border: `1px solid ${c.border}`, borderRadius: 12, padding: '10px', color: c.text, fontSize: 12, fontWeight: 600, fontFamily: sans, cursor: 'pointer', backdropFilter: 'blur(20px)' }}>
            + Nova nota
          </button>
        </div>

        {/* Editor */}
        <div style={{ flex: 1, background: c.glass, backdropFilter: 'blur(40px)', WebkitBackdropFilter: 'blur(40px)', border: `1px solid ${c.border}`, borderRadius: 20, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
          <div style={{ padding: '14px 24px', display: 'flex', alignItems: 'center', gap: 10, borderBottom: `1px solid ${c.border}` }}>
            <span style={{ fontSize: 12, color: c.dim }}>Specs / Motoboys</span>
            <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
              {['⤴', '◐', '⋯'].map(g => (
                <button key={g} style={{ width: 30, height: 30, borderRadius: 9, background: c.glass, border: `1px solid ${c.border}`, color: c.dim, cursor: 'pointer' }}>{g}</button>
              ))}
            </div>
          </div>

          <div style={{ flex: 1, padding: '32px 56px', overflow: 'hidden' }}>
            <div style={{ fontSize: 11, color: c.dimmer, marginBottom: 12, letterSpacing: 2 }}>SPEC · v1.0</div>
            <div style={{ fontSize: 38, fontWeight: 600, letterSpacing: -1.2, lineHeight: 1.1, marginBottom: 14, background: 'linear-gradient(135deg, #fff 0%, #ff6ec7 60%, #6ec1ff 100%)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>
              Módulo Motoboys
            </div>
            <div style={{ color: c.dim, fontSize: 15, marginBottom: 24, maxWidth: 560 }}>
              Linking iFood + app Flutter dedicado para registro de entregas em campo.
            </div>

            <div style={{ display: 'flex', gap: 8, marginBottom: 32, flexWrap: 'wrap' }}>
              <Chip color={c.pink}>cfo-pocket</Chip>
              <Chip color={c.blue}>feat/motoboys</Chip>
              <Chip color={c.green}>active</Chip>
            </div>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 16 }}>
              {[
                ['01', 'Goal', 'Fluxo operacional onde o motoboy registra a entrega via app dedicado.', c.pink],
                ['02', 'Premises', 'iFood ~95% pronto. Sistema funciona se iFood cair.', c.blue],
                ['03', 'Security', 'OWASP Top 10 2021 · zero trust em todos os inputs.', c.green],
                ['04', 'Endpoints', 'Tudo sob /api/v1/motoboy/* — superfície separada.', '#c89eff'],
              ].map(([n, t, d, col]) => (
                <div key={n} style={{ background: c.glass, border: `1px solid ${c.border}`, borderRadius: 16, padding: '18px 20px', backdropFilter: 'blur(20px)' }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 10, marginBottom: 8 }}>
                    <span style={{ fontSize: 11, color: col, fontWeight: 600, letterSpacing: 1 }}>{n}</span>
                    <span style={{ fontSize: 16, fontWeight: 600 }}>{t}</span>
                  </div>
                  <div style={{ fontSize: 13, color: c.dim, lineHeight: 1.55 }}>{d}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
window.Aurora = Aurora;
