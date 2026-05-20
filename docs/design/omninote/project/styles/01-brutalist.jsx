// Brutalist Terminal — TUI/CLI, ASCII borders, monospace
const Brutalist = () => {
  const c = {
    bg: '#0a0a0a',
    panel: '#0f0f0f',
    border: '#2a2a2a',
    text: '#d4d4d4',
    dim: '#6a6a6a',
    amber: '#e8b75a',
    green: '#7ec07e',
    red: '#d4715a',
    blue: '#7aa9d4',
    purple: '#b48ead',
    sel: '#1a1a1a',
  };
  const mono = "'JetBrains Mono', ui-monospace, monospace";
  const rowH = 22;

  const Folder = ({ name, indent = 0, open = true, kind = 'folder' }) => (
    <div style={{ display: 'flex', alignItems: 'center', gap: 6, padding: `0 10px`, height: rowH, color: c.text, paddingLeft: 10 + indent * 14 }}>
      <span style={{ color: c.dim, width: 10 }}>{kind === 'folder' ? (open ? '▾' : '▸') : ' '}</span>
      <span style={{ color: kind === 'folder' ? c.amber : c.dim }}>{kind === 'folder' ? '▣' : '▭'}</span>
      <span>{name}</span>
    </div>
  );

  const Tag = ({ children, color }) => (
    <span style={{ background: color + '22', color, border: `1px solid ${color}55`, padding: '0 6px', fontSize: 12 }}>{children}</span>
  );

  return (
    <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: mono, fontSize: 13, display: 'flex', flexDirection: 'column' }}>
      {/* Title bar */}
      <div style={{ height: 28, background: '#000', borderBottom: `1px solid ${c.border}`, display: 'flex', alignItems: 'center', padding: '0 10px', gap: 8 }}>
        <div style={{ display: 'flex', gap: 6 }}>
          <span style={{ width: 10, height: 10, borderRadius: '50%', background: '#ff5f56' }}></span>
          <span style={{ width: 10, height: 10, borderRadius: '50%', background: '#ffbd2e' }}></span>
          <span style={{ width: 10, height: 10, borderRadius: '50%', background: '#27c93f' }}></span>
        </div>
        <span style={{ marginLeft: 12, color: c.dim, fontSize: 12 }}>OmniNote ▸ ~/cfo-pocket/specs</span>
        <span style={{ marginLeft: 'auto', color: c.dim, fontSize: 12 }}>[brutalist.dark]</span>
      </div>

      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        {/* Sidebar */}
        <div style={{ width: 240, borderRight: `1px solid ${c.border}`, display: 'flex', flexDirection: 'column' }}>
          <div style={{ padding: '8px 10px', borderBottom: `1px solid ${c.border}` }}>
            <div style={{ color: c.dim, fontSize: 11, marginBottom: 4 }}>┌─ search ─────────────┐</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <span style={{ color: c.green }}>$</span>
              <span style={{ color: c.dim }}>grep -r</span>
              <span style={{ color: c.text }}>_</span>
            </div>
          </div>
          <div style={{ padding: '6px 10px', borderBottom: `1px solid ${c.border}`, display: 'flex', flexWrap: 'wrap', gap: 4, fontSize: 11 }}>
            {[['#all', c.amber], ['#spec', c.blue], ['#todo', c.green], ['#draft', c.dim], ['#code', c.purple]].map(([t, col]) => (
              <span key={t} style={{ color: col, padding: '0 4px' }}>{t}</span>
            ))}
          </div>
          <div style={{ flex: 1, overflow: 'hidden', paddingTop: 4 }}>
            <Folder name="cfo-pocket" />
            <Folder name="specs" indent={1} />
            <Folder name="motoboys.md" indent={2} kind="file" />
            <Folder name="ifood-api.md" indent={2} kind="file" />
            <Folder name="focus-nfe.md" indent={2} kind="file" />
            <Folder name="archive" indent={1} open={false} />
            <Folder name="inbox" />
            <Folder name="quick-note.md" indent={1} kind="file" />
            <Folder name="call-log.md" indent={1} kind="file" />
          </div>
          <div style={{ borderTop: `1px solid ${c.border}`, padding: '6px 10px', color: c.dim, fontSize: 11, display: 'flex', gap: 12 }}>
            <span>[n]ew</span><span>[f]older</span><span>[i]mport</span>
          </div>
        </div>

        {/* Editor */}
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          <div style={{ padding: '6px 14px', borderBottom: `1px solid ${c.border}`, display: 'flex', alignItems: 'center', gap: 10, color: c.dim, fontSize: 12 }}>
            <span>≡ EDIT</span>
            <span>·</span>
            <span style={{ color: c.amber }}>motoboys.md</span>
            <span style={{ marginLeft: 'auto' }}>ln 1, col 1 · md · utf-8</span>
          </div>
          <div style={{ flex: 1, padding: '20px 32px', overflow: 'hidden', lineHeight: 1.55 }}>
            <div style={{ color: c.dim, fontSize: 11, marginBottom: 4 }}>┌──────────────────────────────────────────────────────────┐</div>
            <div style={{ color: c.amber, fontWeight: 700, fontSize: 18, letterSpacing: -0.3 }}>SPEC ── MOTOBOYS MODULE</div>
            <div style={{ color: c.dim, fontSize: 11, marginTop: 4 }}>└──────────────────────────────────────────────────────────┘</div>

            <div style={{ marginTop: 14, fontSize: 12, display: 'grid', gridTemplateColumns: 'auto 1fr', gap: '4px 14px' }}>
              <span style={{ color: c.dim }}>project</span><span><Tag color={c.blue}>cfo-pocket</Tag></span>
              <span style={{ color: c.dim }}>branch</span><span><Tag color={c.green}>feat/motoboys</Tag></span>
              <span style={{ color: c.dim }}>owner</span><span>Juan Fausta <span style={{ color: c.dim }}>(CTO)</span></span>
              <span style={{ color: c.dim }}>version</span><span><Tag color={c.amber}>1.0 · 2026-04-20</Tag></span>
            </div>

            <div style={{ marginTop: 22, color: c.green, fontWeight: 700 }}>## 1. GOAL</div>
            <div style={{ marginTop: 8, color: c.text }}>Deliver an operational flow where:</div>
            <ol style={{ margin: '8px 0 0 22px', padding: 0, color: c.text }}>
              <li>The motoboy logs into a dedicated <Tag color={c.purple}>Flutter</Tag> app.</li>
              <li>At delivery time, registers the order by typing the number and selecting the fee tier.</li>
              <li>Backend persists, links to <Tag color={c.amber}>iFood</Tag> when possible.</li>
              <li>Daily checklist consumes the records — no more manual entry.</li>
            </ol>

            <div style={{ marginTop: 22, color: c.green, fontWeight: 700 }}>## 2. PREMISES <span style={{ color: c.dim, fontWeight: 400 }}>(hard constraints)</span></div>
            <ul style={{ margin: '8px 0 0 22px', padding: 0, color: c.text }}>
              <li>iFood integration is ~95% done (branch <Tag color={c.green}>focusNFe</Tag>).</li>
              <li>System MUST continue to work if iFood is unavailable — graceful degradation.</li>
              <li>The Flutter app needs a dedicated motoboy login flow.</li>
              <li>All endpoints under <Tag color={c.blue}>/api/v1/motoboy/*</Tag>.</li>
            </ul>

            <div style={{ marginTop: 22, color: c.green, fontWeight: 700 }}>## 3. SECURITY BASELINE</div>
            <div style={{ marginTop: 8, color: c.dim, fontSize: 12 }}>OWASP Top 10 — 2021 compliance · zero trust on all inputs</div>
          </div>
          <div style={{ borderTop: `1px solid ${c.border}`, padding: '4px 14px', color: c.dim, fontSize: 11, display: 'flex', gap: 14 }}>
            <span style={{ color: c.green }}>● saved</span>
            <span>184 lines</span>
            <span>2.1 kb</span>
            <span style={{ marginLeft: 'auto' }}>UTF-8 · LF · markdown</span>
          </div>
        </div>
      </div>
    </div>
  );
};
window.Brutalist = Brutalist;
