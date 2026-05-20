// Minimal Swiss / Bauhaus — grid, grotesk, single accent, generous whitespace
// Dark mode com acentos laranja, nota selecionada em destaque, e markdown rico
const Swiss = () => {
  const c = {
    bg: '#0e0e0e',
    panel: '#141414',
    border: '#262626',
    borderHot: '#3a2418',
    text: '#fafafa',
    dim: '#8a8a8a',
    dimmer: '#5a5a5a',
    accent: '#ff5a1f',
    accentSoft: '#ff7a47',
    accentBg: 'rgba(255,90,31,0.10)',
    accentBgStrong: 'rgba(255,90,31,0.18)',
  };
  const sans = "'Neue Haas Grotesk', 'Helvetica Neue', 'Inter', -apple-system, sans-serif";
  const mono = "'JetBrains Mono', monospace";

  // Note row in sidebar list — selected uses laranja highlight
  const NoteRow = ({ n, title, meta, tag, active }) => (
    <div style={{
      display: 'grid', gridTemplateColumns: '28px 1fr', columnGap: 10, alignItems: 'baseline',
      padding: '10px 14px',
      background: active ? c.accentBg : 'transparent',
      borderLeft: active ? `2px solid ${c.accent}` : '2px solid transparent',
      borderTop: `1px solid ${active ? c.borderHot : c.border}`,
      cursor: 'pointer',
      position: 'relative',
    }}>
      <span style={{ fontFamily: mono, fontSize: 10, color: active ? c.accent : c.dimmer, letterSpacing: 0.5 }}>{n}</span>
      <div style={{ minWidth: 0 }}>
        <div style={{ fontSize: 13, fontWeight: active ? 600 : 500, color: active ? c.text : c.dim, letterSpacing: -0.1, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{title}</div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 4 }}>
          <span style={{ fontFamily: mono, fontSize: 9, color: active ? c.accentSoft : c.dimmer, letterSpacing: 1 }}>{meta}</span>
          {tag && <span style={{ fontFamily: mono, fontSize: 9, color: active ? c.accent : c.dimmer, letterSpacing: 1, padding: '1px 5px', border: `1px solid ${active ? c.accent : c.border}` }}>{tag}</span>}
        </div>
      </div>
      {active && <span style={{ position: 'absolute', right: 10, top: 12, width: 5, height: 5, background: c.accent, borderRadius: '50%', boxShadow: `0 0 10px ${c.accent}` }}></span>}
    </div>
  );

  const NavItem = ({ n, label, active, count }) => (
    <div style={{
      display: 'grid', gridTemplateColumns: '32px 1fr auto', alignItems: 'baseline',
      padding: '8px 0', borderTop: `1px solid ${c.border}`,
      color: active ? c.text : c.dim, cursor: 'pointer',
    }}>
      <span style={{ fontFamily: mono, fontSize: 10, color: active ? c.accent : c.dimmer, letterSpacing: 0.5 }}>{n}</span>
      <span style={{ fontSize: 13, fontWeight: active ? 600 : 400, letterSpacing: -0.1 }}>{label}</span>
      {count && <span style={{ fontFamily: mono, fontSize: 10, color: c.dimmer }}>{count}</span>}
    </div>
  );

  // Inline markdown elements
  const H = ({ children }) => <span style={{ color: c.accent, fontFamily: mono, marginRight: 6 }}>{children}</span>;
  const Code = ({ children }) => (
    <span style={{ fontFamily: mono, fontSize: 12, color: c.accentSoft, background: c.accentBg, padding: '1px 6px', borderRadius: 2 }}>{children}</span>
  );
  const Tag = ({ children }) => (
    <span style={{ fontFamily: mono, fontSize: 11, color: c.accent, background: c.accentBg, border: `1px solid ${c.accent}55`, padding: '1px 7px', letterSpacing: 0.5 }}>#{children}</span>
  );
  const Link = ({ children }) => (
    <span style={{ color: c.accent, borderBottom: `1px dashed ${c.accent}88`, cursor: 'pointer' }}>{children}</span>
  );

  return (
    <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: sans, display: 'flex' }}>
      {/* Sidebar — wider to show note list */}
      <div style={{ width: 360, borderRight: `1px solid ${c.border}`, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
        {/* Brand */}
        <div style={{ padding: '24px 20px 16px', display: 'flex', alignItems: 'baseline', gap: 8 }}>
          <div style={{ width: 14, height: 14, background: c.accent }}></div>
          <div style={{ fontSize: 16, fontWeight: 700, letterSpacing: -0.4 }}>OmniNote</div>
          <div style={{ marginLeft: 'auto', fontFamily: mono, fontSize: 10, color: c.dimmer }}>26.04</div>
        </div>

        {/* Search box — laranja, com placeholder bracket */}
        <div style={{ padding: '0 20px 18px' }}>
          <div style={{ fontFamily: mono, fontSize: 9, color: c.accent, letterSpacing: 2, marginBottom: 8 }}>BUSCA —</div>
          <div style={{
            display: 'flex', alignItems: 'center', gap: 10,
            border: `1px solid ${c.accent}`,
            background: c.accentBg,
            padding: '8px 12px',
            fontFamily: mono,
          }}>
            <span style={{ color: c.accent, fontSize: 13, fontWeight: 700 }}>busca</span>
            <span style={{ flex: 1, color: c.accent, fontSize: 14, letterSpacing: 1, fontFamily: mono, whiteSpace: 'pre' }}>[                      ]</span>
            <span style={{ fontSize: 10, color: c.accentSoft, letterSpacing: 1 }}>⌘K</span>
          </div>
        </div>

        {/* Views */}
        <div style={{ padding: '0 20px' }}>
          <div style={{ fontFamily: mono, fontSize: 9, color: c.dimmer, letterSpacing: 2, marginBottom: 4 }}>VIEWS —</div>
          <NavItem n="01" label="All" active count="42" />
          <NavItem n="02" label="Drafts" count="07" />
          <NavItem n="03" label="Pinned" count="03" />
        </div>

        {/* Notes list — header */}
        <div style={{ padding: '24px 20px 8px', display: 'flex', alignItems: 'baseline', justifyContent: 'space-between' }}>
          <span style={{ fontFamily: mono, fontSize: 9, color: c.dimmer, letterSpacing: 2 }}>SPECS — 12</span>
          <span style={{ fontFamily: mono, fontSize: 9, color: c.dimmer, letterSpacing: 1 }}>↓ EDITED</span>
        </div>

        <div style={{ flex: 1, overflow: 'hidden', borderBottom: `1px solid ${c.border}` }}>
          <NoteRow n="047" title="Motoboys module" meta="2 MIN AGO" tag="spec" active />
          <NoteRow n="046" title="iFood linking — focusNFe branch" meta="YESTERDAY" tag="api" />
          <NoteRow n="045" title="Daily checklist redesign" meta="2D" tag="ux" />
          <NoteRow n="044" title="OWASP audit — april" meta="3D" tag="sec" />
          <NoteRow n="043" title="Flutter login flow" meta="6D" tag="mobile" />
          <NoteRow n="042" title="Backend persistence layer" meta="1W" tag="db" />
          <NoteRow n="041" title="Field test — Ribeirão" meta="1W" tag="ops" />
          <NoteRow n="040" title="Fee tier matrix" meta="2W" />
        </div>

        {/* New */}
        <div style={{ padding: 20, display: 'flex', alignItems: 'baseline', gap: 12 }}>
          <button style={{ background: c.accent, color: '#000', border: 'none', padding: '10px 18px', fontFamily: sans, fontSize: 12, fontWeight: 700, letterSpacing: 0.3, cursor: 'pointer' }}>+ NEW NOTE</button>
          <span style={{ fontFamily: mono, fontSize: 10, color: c.dimmer }}>⌘N</span>
        </div>
      </div>

      {/* Editor */}
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0, overflow: 'hidden' }}>
        {/* Header bar — laranja stylized */}
        <div style={{
          height: 56,
          background: `linear-gradient(90deg, ${c.accentBgStrong} 0%, transparent 70%)`,
          borderBottom: `1px solid ${c.accent}`,
          display: 'grid', gridTemplateColumns: 'auto 1fr auto', alignItems: 'center',
          padding: '0 32px', gap: 20,
        }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
            <span style={{ fontFamily: mono, fontSize: 18, color: c.accent, fontWeight: 700, letterSpacing: -0.5 }}>{'<html>'}</span>
            <span style={{ fontFamily: mono, fontSize: 10, color: c.accentSoft, letterSpacing: 2 }}>SPECS / 047</span>
          </div>
          <div style={{ fontFamily: mono, fontSize: 11, color: c.text, letterSpacing: 2, fontWeight: 600, textAlign: 'center' }}>
            MOTOBOYS<span style={{ color: c.accent }}>.</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14, fontFamily: mono, fontSize: 10, color: c.accentSoft, letterSpacing: 1.5 }}>
            <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span style={{ width: 5, height: 5, background: c.accent, borderRadius: '50%', boxShadow: `0 0 8px ${c.accent}` }}></span>
              SAVED
            </span>
            <span>EDITED 2 MIN AGO</span>
            <span style={{ color: c.accent }}>⋯</span>
          </div>
        </div>

        {/* Body */}
        <div style={{ flex: 1, padding: '40px 56px', overflow: 'auto', display: 'grid', gridTemplateColumns: '120px 1fr', columnGap: 32 }}>
          {/* Left rail */}
          <div>
            <div style={{ fontFamily: mono, fontSize: 10, color: c.accent, letterSpacing: 2, marginBottom: 4 }}>№ 047</div>
            <div style={{ fontFamily: mono, fontSize: 10, color: c.dimmer, letterSpacing: 2 }}>SPECIFICATION</div>
            <div style={{ width: 40, height: 1, background: c.accent, margin: '14px 0' }}></div>
            <div style={{ fontFamily: mono, fontSize: 10, color: c.dimmer, letterSpacing: 1, lineHeight: 1.8 }}>
              v1.0<br/>
              2026·04·20<br/>
              J. FAUSTA
            </div>
            <div style={{ width: 40, height: 1, background: c.border, margin: '14px 0' }}></div>
            <div style={{ fontFamily: mono, fontSize: 9, color: c.dimmer, letterSpacing: 1.5, lineHeight: 2 }}>
              184 LINES<br/>
              2.1 KB<br/>
              MARKDOWN<br/>
              UTF·8
            </div>
          </div>

          {/* Main markdown content */}
          <div style={{ maxWidth: 720 }}>
            {/* Title */}
            <div style={{ fontSize: 64, fontWeight: 700, letterSpacing: -2.5, lineHeight: 0.95, marginBottom: 8 }}>
              <H># </H>Motoboys<br/>
              <span style={{ color: c.accent }}>module.</span>
            </div>
            <div style={{ fontSize: 17, color: c.dim, fontWeight: 400, letterSpacing: -0.3, marginBottom: 24, maxWidth: 520, lineHeight: 1.4 }}>
              iFood linking, Flutter app, daily checklist automation.
            </div>

            {/* Tags */}
            <div style={{ display: 'flex', gap: 6, marginBottom: 32, flexWrap: 'wrap' }}>
              <Tag>cfo-pocket</Tag>
              <Tag>spec</Tag>
              <Tag>motoboys</Tag>
              <Tag>v1</Tag>
              <Tag>active</Tag>
            </div>

            {/* Stats grid */}
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 0, borderTop: `1px solid ${c.accent}`, borderBottom: `1px solid ${c.accent}`, marginBottom: 36 }}>
              {[
                ['95', '%', 'iFood done'],
                ['1', 'app', 'Flutter'],
                ['10', 'OWASP', 'top'],
                ['v1', '.0', 'release'],
              ].map(([n, u, l]) => (
                <div key={l} style={{ padding: '18px 0 18px 14px', borderLeft: `1px solid ${c.borderHot}` }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 4 }}>
                    <span style={{ fontSize: 36, fontWeight: 700, letterSpacing: -1.5, color: c.accent }}>{n}</span>
                    <span style={{ fontSize: 13, color: c.accentSoft, fontWeight: 500 }}>{u}</span>
                  </div>
                  <div style={{ fontFamily: mono, fontSize: 10, color: c.dimmer, letterSpacing: 1, marginTop: 6 }}>{l.toUpperCase()}</div>
                </div>
              ))}
            </div>

            {/* Section 01 */}
            <div style={{ fontFamily: mono, fontSize: 10, color: c.accent, letterSpacing: 2, marginBottom: 10 }}>01 — GOAL</div>
            <div style={{ fontSize: 26, fontWeight: 600, letterSpacing: -0.8, lineHeight: 1.2, marginBottom: 14 }}>
              <H>##</H>Operational flow for delivery registration
            </div>
            <div style={{ fontSize: 15, lineHeight: 1.65, color: c.text, marginBottom: 14 }}>
              The motoboy logs into a dedicated <Code>Flutter</Code> app — separate from the gestor panel — and registers each delivery by typing the order number and selecting the appropriate fee tier. The backend persists the record and, when possible, links it to an <Link>iFood order</Link>.
            </div>
            <ol style={{ paddingLeft: 0, listStyle: 'none', marginBottom: 32 }}>
              {[
                ['1.', 'Login dedicado no app Flutter (fluxo separado do gestor).'],
                ['2.', 'Registrar entrega: número do pedido + tier de taxa.'],
                ['3.', 'Backend persiste e vincula ao iFood quando possível.'],
                ['4.', 'Daily checklist consome os registros — sem entrada manual.'],
              ].map(([n, t]) => (
                <li key={n} style={{ display: 'grid', gridTemplateColumns: '32px 1fr', gap: 8, padding: '6px 0', fontSize: 14, color: c.text, lineHeight: 1.55 }}>
                  <span style={{ fontFamily: mono, color: c.accent, fontWeight: 700 }}>{n}</span>
                  <span>{t}</span>
                </li>
              ))}
            </ol>

            {/* Section 02 */}
            <div style={{ fontFamily: mono, fontSize: 10, color: c.accent, letterSpacing: 2, marginBottom: 10 }}>02 — PREMISES</div>
            <div style={{ fontSize: 26, fontWeight: 600, letterSpacing: -0.8, lineHeight: 1.2, marginBottom: 14 }}>
              <H>##</H>Hard constraints
            </div>
            <ul style={{ paddingLeft: 0, listStyle: 'none', marginBottom: 28 }}>
              {[
                ['→', <>iFood integration ~95% done on branch <Code>focusNFe</Code>.</>],
                ['→', <>System MUST continue to work if iFood is unavailable — graceful degradation.</>],
                ['→', <>Flutter app needs a dedicated motoboy login flow.</>],
                ['→', <>All endpoints under <Code>/api/v1/motoboy/*</Code>.</>],
              ].map(([m, t], i) => (
                <li key={i} style={{ display: 'grid', gridTemplateColumns: '24px 1fr', gap: 8, padding: '6px 0', fontSize: 14, color: c.text, lineHeight: 1.6, borderTop: i === 0 ? 'none' : `1px solid ${c.border}` }}>
                  <span style={{ color: c.accent, fontFamily: mono }}>{m}</span>
                  <span>{t}</span>
                </li>
              ))}
            </ul>

            {/* Blockquote — laranja */}
            <div style={{
              borderLeft: `3px solid ${c.accent}`,
              background: c.accentBg,
              padding: '14px 20px',
              fontFamily: mono, fontSize: 13, color: c.accentSoft, lineHeight: 1.65,
              marginBottom: 32,
            }}>
              <span style={{ color: c.accent, fontWeight: 700 }}>{'>'}</span> "OWASP Top 10 — 2021 compliance · zero trust on all inputs."
              <div style={{ marginTop: 6, fontSize: 10, color: c.dimmer, letterSpacing: 1.5 }}>— SECURITY BASELINE</div>
            </div>

            {/* Code block */}
            <div style={{ fontFamily: mono, fontSize: 10, color: c.accent, letterSpacing: 2, marginBottom: 10 }}>03 — ENDPOINTS</div>
            <div style={{
              border: `1px solid ${c.borderHot}`,
              background: '#0a0a0a',
              fontFamily: mono, fontSize: 12,
              padding: '14px 18px',
              lineHeight: 1.8,
              marginBottom: 32,
            }}>
              <div><span style={{ color: c.accent }}>POST</span>   <span style={{ color: c.accentSoft }}>/api/v1/motoboy/login</span>          <span style={{ color: c.dimmer }}>// auth</span></div>
              <div><span style={{ color: c.accent }}>POST</span>   <span style={{ color: c.accentSoft }}>/api/v1/motoboy/deliveries</span>     <span style={{ color: c.dimmer }}>// register</span></div>
              <div><span style={{ color: c.accent }}>GET</span>    <span style={{ color: c.accentSoft }}>/api/v1/motoboy/deliveries/today</span><span style={{ color: c.dimmer }}> // list</span></div>
              <div><span style={{ color: c.accent }}>PATCH</span>  <span style={{ color: c.accentSoft }}>/api/v1/motoboy/deliveries/:id</span>  <span style={{ color: c.dimmer }}>// link iFood</span></div>
            </div>

            {/* Footer meta */}
            <div style={{ borderTop: `1px solid ${c.border}`, paddingTop: 16, display: 'flex', gap: 18, fontFamily: mono, fontSize: 10, color: c.dimmer, letterSpacing: 1.5 }}>
              <span>BACKLINKS — <span style={{ color: c.accent }}>3</span></span>
              <span>WORDS — <span style={{ color: c.accent }}>284</span></span>
              <span style={{ marginLeft: 'auto', color: c.accent }}>{'</html>'}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
window.Swiss = Swiss;
