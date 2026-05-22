// Editorial Serif Dark — magazine, generous type, serif + sans
const Editorial = () => {
  const c = {
    bg: '#1a1816',
    panel: '#211e1b',
    border: '#2e2a26',
    text: '#ede8df',
    dim: '#a39b8e',
    dimmer: '#6b6359',
    accent: '#d97757',  // warm terracotta
    rule: '#3a342e',
  };
  const serif = "'Source Serif 4', 'Source Serif Pro', Georgia, serif";
  const sans = "'Inter', -apple-system, sans-serif";

  const NavItem = ({ label, active, meta }) => (
    <div style={{
      display: 'flex', alignItems: 'baseline', gap: 10, padding: '7px 0',
      color: active ? c.text : c.dim, fontFamily: serif, fontSize: 14,
      borderLeft: active ? `2px solid ${c.accent}` : `2px solid transparent`,
      paddingLeft: 14, cursor: 'pointer',
      fontStyle: active ? 'italic' : 'normal',
    }}>
      <span style={{ flex: 1 }}>{label}</span>
      {meta && <span style={{ fontFamily: sans, fontSize: 10, color: c.dimmer, letterSpacing: 1 }}>{meta}</span>}
    </div>
  );

  return (
    <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, display: 'flex' }}>
      {/* Sidebar */}
      <div style={{ width: 280, borderRight: `1px solid ${c.rule}`, padding: '28px 24px', display: 'flex', flexDirection: 'column' }}>
        <div style={{ marginBottom: 28 }}>
          <div style={{ fontFamily: serif, fontSize: 24, fontStyle: 'italic', letterSpacing: -0.5 }}>OmniNote</div>
          <div style={{ fontFamily: sans, fontSize: 10, color: c.dimmer, letterSpacing: 2, marginTop: 2 }}>EST. MMXXVI · VOL. 01</div>
        </div>

        <div style={{ borderTop: `1px solid ${c.rule}`, borderBottom: `1px solid ${c.rule}`, padding: '12px 0', marginBottom: 18, display: 'flex', alignItems: 'baseline', gap: 8 }}>
          <span style={{ fontFamily: sans, fontSize: 10, color: c.dimmer, letterSpacing: 2 }}>SEARCH</span>
          <span style={{ fontFamily: serif, fontStyle: 'italic', color: c.dim, fontSize: 13 }}>type to find…</span>
        </div>

        <div style={{ fontFamily: sans, fontSize: 10, color: c.dimmer, letterSpacing: 2.5, marginBottom: 8 }}>SECTIONS</div>
        <NavItem label="All entries" active meta="42" />
        <NavItem label="Drafts" meta="07" />
        <NavItem label="Pinned" meta="03" />
        <NavItem label="Archive" />

        <div style={{ fontFamily: sans, fontSize: 10, color: c.dimmer, letterSpacing: 2.5, margin: '28px 0 8px' }}>VOLUMES</div>
        <NavItem label="Specifications" meta="XII" />
        <NavItem label="Meeting notes" meta="VIII" />
        <NavItem label="Daily logs" meta="XXX" />
        <NavItem label="Letters" meta="V" />

        <div style={{ marginTop: 'auto', borderTop: `1px solid ${c.rule}`, paddingTop: 14, fontFamily: serif, fontStyle: 'italic', fontSize: 12, color: c.dimmer, lineHeight: 1.5 }}>
          “Write hard and clear about what hurts.”
          <div style={{ fontFamily: sans, fontStyle: 'normal', fontSize: 10, color: c.dimmer, letterSpacing: 1, marginTop: 4 }}>— EH</div>
        </div>
      </div>

      {/* Editor */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
        <div style={{ padding: '20px 56px 16px', borderBottom: `1px solid ${c.rule}`, display: 'flex', alignItems: 'baseline', gap: 16 }}>
          <span style={{ fontFamily: sans, fontSize: 10, color: c.dimmer, letterSpacing: 2 }}>SPECIFICATIONS · NO. 047</span>
          <span style={{ fontFamily: serif, fontStyle: 'italic', color: c.dim, fontSize: 13, marginLeft: 'auto' }}>April XX, MMXXVI</span>
        </div>

        <div style={{ flex: 1, padding: '48px 56px', overflow: 'hidden', maxWidth: 820 }}>
          <div style={{ fontFamily: sans, fontSize: 10, color: c.accent, letterSpacing: 3, marginBottom: 12 }}>FEATURED</div>
          <div style={{ fontFamily: serif, fontSize: 56, fontWeight: 400, lineHeight: 1.05, letterSpacing: -1.5, marginBottom: 16 }}>
            The <span style={{ fontStyle: 'italic' }}>Motoboys</span> Module
          </div>
          <div style={{ fontFamily: serif, fontSize: 18, fontStyle: 'italic', color: c.dim, lineHeight: 1.4, marginBottom: 22, maxWidth: 580 }}>
            Bridging iFood orders, Flutter field clients, and the daily expense checklist — a quiet redesign of the delivery loop.
          </div>

          <div style={{ display: 'flex', alignItems: 'baseline', gap: 14, fontFamily: sans, fontSize: 11, color: c.dimmer, letterSpacing: 1, paddingBottom: 18, borderBottom: `1px solid ${c.rule}`, marginBottom: 28 }}>
            <span>BY <span style={{ color: c.text }}>JUAN FAUSTA</span></span>
            <span>·</span>
            <span>FOR CFO POCKET</span>
            <span>·</span>
            <span>READ TIME 6 MIN</span>
          </div>

          <div style={{ columnCount: 2, columnGap: 32, fontFamily: serif, fontSize: 15, lineHeight: 1.65, color: c.text }}>
            <p style={{ margin: 0 }}>
              <span style={{ float: 'left', fontSize: 64, lineHeight: 0.9, paddingRight: 8, paddingTop: 6, color: c.accent, fontWeight: 600 }}>D</span>
              eliver an operational flow where the motoboy logs into a dedicated Flutter application — separate from the gestor panel — and registers each delivery by typing the order number and selecting the appropriate fee tier.
            </p>
            <p>
              The backend persists the delivery and, when possible, links it to an iFood order. The daily checklist consumes those records automatically, retiring the practice of manual entry that has slowed the closing routine for months.
            </p>
            <p style={{ fontFamily: sans, fontSize: 10, letterSpacing: 2, color: c.accent, margin: '18px 0 6px' }}>§ PREMISES</p>
            <p style={{ margin: 0 }}>
              The iFood integration sits at roughly ninety-five percent completion on the <span style={{ fontStyle: 'italic' }}>focusNFe</span> branch. The system must continue to function gracefully when iFood is unreachable — falling back to the classical manual order number with no loss of fidelity.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
};
window.Editorial = Editorial;
