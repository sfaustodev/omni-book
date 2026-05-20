// Soft Neumorphic Dark — elevated surfaces, soft shadows, rounded
const Neumorphic = () => {
  const c = {
    bg: '#1e2024',
    panel: '#23262b',
    raised: '#272a30',
    text: '#e8eaee',
    dim: '#8c919b',
    accent: '#7c9cf5',
    coral: '#f59e7c',
    mint: '#7cdfb6',
  };
  const sans = "'Inter Tight', 'Inter', -apple-system, sans-serif";
  const insetShadow = `inset 4px 4px 8px #16181b, inset -4px -4px 8px #2a2d33`;
  const raisedShadow = `6px 6px 14px #16181b, -6px -6px 14px #2a2d33`;
  const softShadow = `3px 3px 7px #16181b, -3px -3px 7px #2a2d33`;

  const NavItem = ({ icon, label, active, count }) => (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 12, padding: '11px 14px', borderRadius: 14,
      background: active ? c.bg : 'transparent',
      boxShadow: active ? insetShadow : 'none',
      color: active ? c.text : c.dim,
      fontSize: 13, fontWeight: active ? 600 : 500, cursor: 'pointer',
    }}>
      <span style={{ width: 18, fontSize: 14, color: active ? c.accent : c.dim }}>{icon}</span>
      <span style={{ flex: 1 }}>{label}</span>
      {count && <span style={{ fontSize: 11, color: c.dim }}>{count}</span>}
    </div>
  );

  const Pill = ({ children, color }) => (
    <span style={{
      display: 'inline-block', padding: '3px 10px', borderRadius: 8,
      background: c.bg, boxShadow: insetShadow,
      color, fontSize: 11, fontWeight: 600, letterSpacing: 0.2,
    }}>{children}</span>
  );

  return (
    <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: sans, display: 'flex', padding: 18, gap: 18 }}>
      {/* Sidebar */}
      <div style={{ width: 260, background: c.panel, borderRadius: 22, boxShadow: raisedShadow, padding: 16, display: 'flex', flexDirection: 'column' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, padding: '4px 8px 14px' }}>
          <div style={{ width: 32, height: 32, borderRadius: 10, background: c.bg, boxShadow: softShadow, display: 'grid', placeItems: 'center', color: c.accent, fontWeight: 700 }}>◐</div>
          <div>
            <div style={{ fontSize: 13, fontWeight: 700 }}>OmniNote</div>
            <div style={{ fontSize: 10, color: c.dim }}>cfo-pocket workspace</div>
          </div>
        </div>

        <div style={{ background: c.bg, borderRadius: 14, boxShadow: insetShadow, padding: '10px 14px', display: 'flex', alignItems: 'center', gap: 8, marginBottom: 14 }}>
          <span style={{ color: c.dim, fontSize: 13 }}>⌕</span>
          <span style={{ color: c.dim, fontSize: 12 }}>Buscar notas…</span>
          <span style={{ marginLeft: 'auto', fontSize: 10, color: c.dim, padding: '2px 6px', borderRadius: 5, background: c.panel, boxShadow: softShadow }}>⌘K</span>
        </div>

        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: 1.2, color: c.dim, padding: '8px 14px 6px' }}>WORKSPACE</div>
        <NavItem icon="◇" label="Todas as notas" active count="42" />
        <NavItem icon="✎" label="Rascunhos" count="7" />
        <NavItem icon="★" label="Favoritos" count="3" />

        <div style={{ fontSize: 10, fontWeight: 700, letterSpacing: 1.2, color: c.dim, padding: '14px 14px 6px' }}>PASTAS</div>
        <NavItem icon="▦" label="Specs" />
        <NavItem icon="▦" label="Reuniões" />
        <NavItem icon="▦" label="Daily" />

        <div style={{ marginTop: 'auto', display: 'flex', gap: 8 }}>
          <button style={{ flex: 1, background: c.bg, boxShadow: softShadow, border: 'none', borderRadius: 12, padding: '10px', color: c.text, fontSize: 12, fontWeight: 600, fontFamily: sans, cursor: 'pointer' }}>+ Nota</button>
          <button style={{ width: 40, background: c.bg, boxShadow: softShadow, border: 'none', borderRadius: 12, color: c.dim, cursor: 'pointer' }}>⤓</button>
        </div>
      </div>

      {/* Editor */}
      <div style={{ flex: 1, background: c.panel, borderRadius: 22, boxShadow: raisedShadow, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
        <div style={{ padding: '18px 28px', display: 'flex', alignItems: 'center', gap: 10 }}>
          <div style={{ display: 'flex', gap: 6 }}>
            <span style={{ fontSize: 12, color: c.dim }}>Specs</span>
            <span style={{ fontSize: 12, color: c.dim }}>›</span>
            <span style={{ fontSize: 12, color: c.text, fontWeight: 600 }}>Motoboys</span>
          </div>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 8 }}>
            {['◐', '⤴', '⋯'].map(g => (
              <button key={g} style={{ width: 32, height: 32, borderRadius: 10, background: c.bg, boxShadow: softShadow, border: 'none', color: c.dim, cursor: 'pointer' }}>{g}</button>
            ))}
          </div>
        </div>

        <div style={{ flex: 1, padding: '4px 56px 24px', overflow: 'hidden' }}>
          <div style={{ fontSize: 32, fontWeight: 700, letterSpacing: -0.8, marginBottom: 6 }}>Spec — Módulo Motoboys</div>
          <div style={{ color: c.dim, fontSize: 14, marginBottom: 18 }}>iFood linking + Flutter app · v1.0 · 20 Apr 2026</div>

          <div style={{ display: 'flex', gap: 8, marginBottom: 24, flexWrap: 'wrap' }}>
            <Pill color={c.accent}>cfo-pocket</Pill>
            <Pill color={c.mint}>feat/motoboys</Pill>
            <Pill color={c.coral}>owner: Juan</Pill>
          </div>

          <div style={{ background: c.bg, borderRadius: 16, boxShadow: insetShadow, padding: '18px 22px', marginBottom: 18 }}>
            <div style={{ fontSize: 13, fontWeight: 700, color: c.accent, marginBottom: 8, letterSpacing: 0.3 }}>1 · OBJETIVO</div>
            <div style={{ color: c.text, fontSize: 14, lineHeight: 1.6 }}>
              Entregar um fluxo operacional onde o motoboy faz login em um app Flutter dedicado, registra a entrega digitando o número do pedido e selecionando o tier de taxa, e o backend persiste — vinculando ao iFood quando possível.
            </div>
          </div>

          <div style={{ background: c.bg, borderRadius: 16, boxShadow: insetShadow, padding: '18px 22px' }}>
            <div style={{ fontSize: 13, fontWeight: 700, color: c.coral, marginBottom: 12, letterSpacing: 0.3 }}>2 · PREMISSAS</div>
            {[
              ['Integração iFood ~95% concluída', 'feat/focusNFe'],
              ['Sistema deve funcionar offline', 'fallback manual'],
              ['App Flutter — fluxo de login dedicado', 'separado do gestor'],
            ].map(([t, sub]) => (
              <div key={t} style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '8px 0', borderTop: `1px solid #1a1c20` }}>
                <span style={{ width: 18, height: 18, borderRadius: 6, background: c.panel, boxShadow: softShadow, display: 'grid', placeItems: 'center', color: c.mint, fontSize: 11 }}>✓</span>
                <span style={{ flex: 1, fontSize: 13 }}>{t}</span>
                <Pill color={c.dim}>{sub}</Pill>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
window.Neumorphic = Neumorphic;
