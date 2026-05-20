// OmniNote — Obsidian-class three-pane notebook UI, egui-realistic
// Original design — dark, single accent, modest rounding, 1px borders, no fancy CSS
// Designed for egui 0.29 immediate-mode rendering

const c = {
  // Workspace
  bg: '#1b1d22',          // editor canvas
  chrome: '#171a1e',      // title bar / status bar
  panel: '#212429',       // sidebar / right rail
  panel2: '#272a30',      // raised: tab strip, hovered rows
  inset: '#15171b',       // search field, code blocks
  border: '#2d3037',
  borderStrong: '#373b44',
  divider: '#23262c',

  // Text
  text: '#dadde2',
  textStrong: '#f0f2f5',
  dim: '#8b8f98',
  dimmer: '#5e6470',

  // Accent (configurable in AppConfig — default soft violet, original)
  accent: '#8b7cff',
  accentDim: '#5b4fc4',
  accentBg: 'rgba(139,124,255,0.12)',
  accentBgStrong: 'rgba(139,124,255,0.20)',

  // Semantic
  green: '#5dcc8f',
  amber: '#e5b14a',
  red: '#e5715a',
  blue: '#5fb2f0',
  pink: '#e07cc4',
};

const sans = "'Inter', -apple-system, sans-serif";
const mono = "'JetBrains Mono', monospace";

// Per NoteType — small typed glyph + tint
const typeColor = {
  resumo: c.blue,
  citacao: c.pink,
  codigo: c.green,
  exercicio: c.amber,
  duvida: c.red,
  definicao: c.accent,
};
const typeGlyph = {
  resumo: '▤',
  citacao: '"',
  codigo: '⌘',
  exercicio: '◇',
  duvida: '?',
  definicao: '§',
};

// ────────────── Shared atoms ──────────────

const Icon = ({ children, size = 14, color = c.dim }) => (
  <span style={{ display: 'inline-block', width: size, height: size, lineHeight: `${size}px`, textAlign: 'center', fontFamily: mono, fontSize: size - 1, color, flexShrink: 0 }}>{children}</span>
);

const IconBtn = ({ children, size = 26, active }) => (
  <button style={{
    width: size, height: size, border: 'none', background: active ? c.accentBg : 'transparent',
    color: active ? c.accent : c.dim, borderRadius: 4, cursor: 'pointer',
    display: 'grid', placeItems: 'center', fontFamily: mono, fontSize: 13,
  }}>{children}</button>
);

const Kbd = ({ children }) => (
  <span style={{
    fontFamily: mono, fontSize: 10, color: c.dim,
    border: `1px solid ${c.border}`, borderBottomWidth: 2,
    padding: '1px 5px', borderRadius: 3, background: c.chrome,
  }}>{children}</span>
);

const TypeChip = ({ kind, count, active }) => (
  <button style={{
    display: 'flex', alignItems: 'center', gap: 5,
    padding: '2px 8px', border: `1px solid ${active ? typeColor[kind] : c.border}`,
    background: active ? `${typeColor[kind]}1f` : 'transparent',
    color: active ? typeColor[kind] : c.dim,
    borderRadius: 999, fontSize: 11, fontFamily: sans, cursor: 'pointer',
  }}>
    <span style={{ color: typeColor[kind], fontFamily: mono, fontSize: 11 }}>{typeGlyph[kind]}</span>
    <span>{kind}</span>
    {count != null && <span style={{ color: c.dimmer, fontSize: 10 }}>{count}</span>}
  </button>
);

// ────────────── Sidebar ──────────────

const SectionHeader = ({ label, count, action = '+' }) => (
  <div style={{
    display: 'flex', alignItems: 'center', gap: 6,
    padding: '14px 14px 6px',
    fontFamily: sans, fontSize: 10, color: c.dimmer,
    letterSpacing: 1.5, fontWeight: 600, textTransform: 'uppercase',
  }}>
    <Icon size={10} color={c.dimmer}>▾</Icon>
    <span>{label}</span>
    {count != null && <span style={{ color: c.dimmer, fontWeight: 400 }}>{count}</span>}
    <span style={{ marginLeft: 'auto', color: c.dimmer, cursor: 'pointer' }}>{action}</span>
  </div>
);

const FileRow = ({ name, indent = 0, kind = 'file', icon, active, badge, badgeColor, color, dim }) => (
  <div style={{
    display: 'flex', alignItems: 'center', gap: 6,
    padding: '4px 14px', paddingLeft: 14 + indent * 14,
    background: active ? c.accentBg : 'transparent',
    borderLeft: active ? `2px solid ${c.accent}` : '2px solid transparent',
    color: active ? c.textStrong : (color || c.text), fontFamily: sans, fontSize: 13,
    cursor: 'pointer', height: 26,
  }}>
    {kind === 'folder' ? (
      <Icon size={11} color={c.dim}>▸</Icon>
    ) : <span style={{ width: 11 }}></span>}
    <Icon size={12} color={active ? c.accent : (color || c.dim)}>{icon || (kind === 'folder' ? '▦' : '◌')}</Icon>
    <span style={{ flex: 1, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', opacity: dim ? 0.55 : 1 }}>{name}</span>
    {badge && (
      <span style={{
        fontFamily: mono, fontSize: 9, padding: '0 5px', borderRadius: 8,
        background: badgeColor || c.accent, color: '#0a0a0a', fontWeight: 700,
      }}>{badge}</span>
    )}
  </div>
);

const Sidebar = () => (
  <div style={{ width: 280, background: c.panel, borderRight: `1px solid ${c.border}`, display: 'flex', flexDirection: 'column', minHeight: 0, fontFamily: sans }}>
    {/* Vault switcher */}
    <div style={{ padding: '10px 12px', borderBottom: `1px solid ${c.border}`, display: 'flex', alignItems: 'center', gap: 8 }}>
      <div style={{ width: 22, height: 22, borderRadius: 5, background: `linear-gradient(135deg, ${c.accent}, ${c.accentDim})`, display: 'grid', placeItems: 'center', color: '#0a0a0a', fontWeight: 800, fontSize: 12 }}>O</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 12, fontWeight: 600, color: c.textStrong, display: 'flex', alignItems: 'center', gap: 4 }}>
          ClaudeBook <span style={{ color: c.dim, fontSize: 10 }}>▾</span>
        </div>
        <div style={{ fontSize: 10, color: c.dimmer, fontFamily: mono }}>~/Projects/caderno</div>
      </div>
      <IconBtn>⚙</IconBtn>
    </div>

    {/* Search */}
    <div style={{ padding: '10px 12px' }}>
      <div style={{
        display: 'flex', alignItems: 'center', gap: 8,
        background: c.inset, border: `1px solid ${c.border}`, borderRadius: 6,
        padding: '6px 10px',
      }}>
        <Icon size={12} color={c.dim}>⌕</Icon>
        <span style={{ flex: 1, color: c.dim, fontSize: 12 }}>Buscar no vault…</span>
        <Kbd>⌘P</Kbd>
      </div>
    </div>

    {/* Type filter chips */}
    <div style={{ padding: '0 12px 8px', display: 'flex', flexWrap: 'wrap', gap: 4 }}>
      <TypeChip kind="resumo" count={18} active />
      <TypeChip kind="codigo" count={9} />
      <TypeChip kind="duvida" count={4} />
      <TypeChip kind="definicao" count={11} />
      <TypeChip kind="citacao" count={6} />
    </div>

    {/* Scroll body */}
    <div style={{ flex: 1, overflow: 'hidden', borderTop: `1px solid ${c.divider}` }}>
      {/* Daily — pinned */}
      <SectionHeader label="Today" action="📅" />
      <FileRow name="2026-05-20.md" icon="◉" active color={c.accent} />
      <FileRow name="2026-05-19.md" icon="○" />
      <FileRow name="2026-05-18.md" icon="○" />
      <div style={{ padding: '2px 14px 6px 36px', fontSize: 11, color: c.dimmer, fontFamily: mono, cursor: 'pointer' }}>open calendar…</div>

      {/* Inbox */}
      <SectionHeader label="Inbox" />
      <FileRow name="Inbox.md" icon="▼" badge="3" badgeColor={c.accent} />

      {/* Discipline */}
      <SectionHeader label="Disciplines" count={6} />
      <FileRow name="SPRINT.md" icon="◈" color={c.green} />
      <FileRow name="DIARY.md" icon="✎" color={c.amber} />
      <FileRow name="JIRA.md" icon="◧" color={c.blue} />
      <FileRow name="HUMAN.md" icon="☻" color={c.pink} />
      <FileRow name="PLAN.md" icon="≡" color={c.accent} />
      <FileRow name="NOTION.md" icon="◨" color={c.dim} />

      {/* Projects */}
      <SectionHeader label="Projects" count={4} />
      <FileRow name="caderno" kind="folder" icon="◆" indent={0} />
      <FileRow name="cfo-pocket" kind="folder" icon="◆" indent={0} />
      <FileRow name="omni-mcp" kind="folder" icon="◆" indent={0} />
      <FileRow name="kappa-trade" kind="folder" icon="◆" indent={0} dim />

      {/* Vault tree */}
      <SectionHeader label="Vault" count={142} />
      <FileRow name="Templates" kind="folder" indent={0} />
      <FileRow name="_attachments" kind="folder" indent={0} dim />
      <FileRow name="01 — Notes" kind="folder" indent={0} />
      <FileRow name="egui patterns.md" indent={1} icon={typeGlyph.codigo} color={typeColor.codigo} />
      <FileRow name="rust borrow checker.md" indent={1} icon={typeGlyph.definicao} color={typeColor.definicao} />
      <FileRow name="Whisper benchmarks.md" indent={1} icon={typeGlyph.resumo} color={typeColor.resumo} />
      <FileRow name="02 — Reading" kind="folder" indent={0} />
      <FileRow name="03 — Refs" kind="folder" indent={0} />
    </div>

    {/* Footer */}
    <div style={{ borderTop: `1px solid ${c.border}`, padding: '6px 10px', display: 'flex', alignItems: 'center', gap: 4 }}>
      <IconBtn>＋</IconBtn>
      <IconBtn active>⧉</IconBtn>
      <IconBtn>♯</IconBtn>
      <IconBtn>◷</IconBtn>
      <span style={{ marginLeft: 'auto', fontFamily: mono, fontSize: 10, color: c.dimmer }}>142 notes · 3.2MB</span>
    </div>
  </div>
);

// ────────────── Editor ──────────────

const Tab = ({ title, kind, active, dirty, onClose }) => (
  <div style={{
    display: 'flex', alignItems: 'center', gap: 7,
    padding: '0 10px 0 12px', height: 32,
    background: active ? c.bg : 'transparent',
    borderRight: `1px solid ${c.border}`,
    borderTop: active ? `1px solid ${c.accent}` : '1px solid transparent',
    color: active ? c.textStrong : c.dim, fontSize: 12, cursor: 'pointer',
    fontFamily: sans,
  }}>
    <Icon size={11} color={typeColor[kind] || c.dim}>{typeGlyph[kind] || '◌'}</Icon>
    <span style={{ maxWidth: 140, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{title}</span>
    {dirty && <span style={{ width: 5, height: 5, background: c.accent, borderRadius: '50%' }}></span>}
    <span style={{ color: c.dimmer, marginLeft: 2 }}>×</span>
  </div>
);

// Inline markdown atoms (rendered live in egui via custom layout)
const WikiLink = ({ children, broken }) => (
  <span style={{
    color: broken ? c.red : c.accent,
    borderBottom: broken ? `1px dashed ${c.red}` : `1px solid transparent`,
    cursor: 'pointer',
  }}>{children}</span>
);
const HashTag = ({ children }) => (
  <span style={{
    color: c.accent, background: c.accentBg,
    padding: '0 6px', borderRadius: 3, fontSize: 13,
    cursor: 'pointer',
  }}>#{children}</span>
);
const InlineCode = ({ children }) => (
  <span style={{ fontFamily: mono, fontSize: 12.5, background: c.inset, color: c.amber, padding: '1px 5px', borderRadius: 3, border: `1px solid ${c.border}` }}>{children}</span>
);

const Editor = () => (
  <div style={{ flex: 1, background: c.bg, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
    {/* Tab strip */}
    <div style={{ height: 32, background: c.panel2, borderBottom: `1px solid ${c.border}`, display: 'flex', alignItems: 'stretch' }}>
      <Tab title="2026-05-20.md" kind="resumo" active dirty />
      <Tab title="egui patterns.md" kind="codigo" />
      <Tab title="SPRINT.md" kind="resumo" />
      <div style={{ flex: 1 }}></div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 2, padding: '0 8px' }}>
        <IconBtn>↶</IconBtn>
        <IconBtn>↷</IconBtn>
        <IconBtn>⊞</IconBtn>
      </div>
    </div>

    {/* Breadcrumb */}
    <div style={{ height: 30, padding: '0 24px', display: 'flex', alignItems: 'center', gap: 8, fontFamily: sans, fontSize: 11, color: c.dim, borderBottom: `1px solid ${c.divider}` }}>
      <span>Daily</span><span style={{ color: c.dimmer }}>/</span>
      <span>2026</span><span style={{ color: c.dimmer }}>/</span>
      <span style={{ color: c.text }}>2026-05-20</span>
      <span style={{ color: c.dimmer }}>›</span>
      <span style={{ color: c.text }}>Sprint review</span>
      <span style={{ marginLeft: 'auto', color: c.dimmer, fontFamily: mono }}>edited 2m ago · 412 words</span>
      <IconBtn>👁</IconBtn>
      <IconBtn active>✎</IconBtn>
      <IconBtn>⋯</IconBtn>
    </div>

    {/* Content scroll */}
    <div style={{ flex: 1, overflow: 'hidden', position: 'relative' }}>
      <div style={{ padding: '36px 64px 36px 80px', maxWidth: 820 }}>
        {/* Frontmatter */}
        <div style={{
          fontFamily: mono, fontSize: 11.5, lineHeight: 1.8,
          background: c.inset, border: `1px solid ${c.border}`, borderLeft: `2px solid ${c.accent}`,
          padding: '10px 14px', borderRadius: 4, marginBottom: 24, color: c.dim,
        }}>
          <div><span style={{ color: c.accent }}>---</span></div>
          <div><span style={{ color: c.amber }}>type:</span> <span style={{ color: c.green }}>daily</span></div>
          <div><span style={{ color: c.amber }}>aliases:</span> [Today, Quarta]</div>
          <div><span style={{ color: c.amber }}>tags:</span> [#sprint, #daily, #cfo-pocket]</div>
          <div><span style={{ color: c.accent }}>---</span></div>
        </div>

        {/* H1 */}
        <h1 style={{ fontSize: 32, fontWeight: 700, color: c.textStrong, letterSpacing: -0.6, margin: '0 0 4px', position: 'relative' }}>
          <span style={{ position: 'absolute', left: -32, color: c.dimmer, fontFamily: mono, fontSize: 18, top: 8 }}>#</span>
          2026-05-20 · Quarta
        </h1>
        <div style={{ color: c.dim, fontSize: 13, marginBottom: 24, display: 'flex', alignItems: 'center', gap: 12 }}>
          <span>← <WikiLink>2026-05-19</WikiLink></span>
          <span style={{ color: c.dimmer }}>·</span>
          <span><WikiLink>2026-05-21</WikiLink> →</span>
          <span style={{ marginLeft: 'auto' }}><HashTag>daily</HashTag> <HashTag>sprint</HashTag></span>
        </div>

        {/* H2 */}
        <h2 style={{ fontSize: 21, fontWeight: 600, color: c.textStrong, margin: '0 0 10px', position: 'relative' }}>
          <span style={{ position: 'absolute', left: -32, color: c.dimmer, fontFamily: mono, fontSize: 14, top: 5 }}>##</span>
          Sprint review
        </h2>
        <p style={{ fontSize: 14.5, lineHeight: 1.7, color: c.text, margin: '0 0 14px' }}>
          Fechamos a integração <WikiLink>iFood Linking</WikiLink> no branch <InlineCode>feat/focusNFe</InlineCode>. O fluxo do <WikiLink>Motoboys module</WikiLink> está em <HashTag>review</HashTag> — falta o passo de OWASP audit (ver <WikiLink>OWASP-2021 checklist</WikiLink>). Quebrei o registro de entrega em três rotas: <InlineCode>POST /api/v1/motoboy/deliveries</InlineCode>, <InlineCode>GET …/today</InlineCode> e <InlineCode>PATCH …/:id</InlineCode>.
        </p>
        <p style={{ fontSize: 14.5, lineHeight: 1.7, color: c.text, margin: '0 0 18px' }}>
          Conversei com a Ju sobre o <WikiLink broken>Fee Tier Matrix</WikiLink> — ainda preciso criar essa nota. Para o app Flutter, ver discussão em <WikiLink>HUMAN#login-flow</WikiLink>.
        </p>

        {/* Embedded note card */}
        <div style={{
          border: `1px solid ${c.border}`, borderRadius: 6, background: c.panel,
          padding: '12px 16px', margin: '6px 0 18px', position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
            <Icon size={12} color={c.blue}>{typeGlyph.resumo}</Icon>
            <span style={{ fontSize: 12, color: c.dim, fontFamily: mono }}>!![[Motoboys module]]</span>
            <span style={{ marginLeft: 'auto', fontSize: 11, color: c.accent, cursor: 'pointer' }}>open full →</span>
          </div>
          <div style={{ fontSize: 15, fontWeight: 600, color: c.textStrong, marginBottom: 4 }}>Motoboys module — spec v1.0</div>
          <div style={{ fontSize: 13, color: c.dim, lineHeight: 1.55 }}>
            Entregar um fluxo operacional onde o motoboy faz login num app Flutter dedicado, registra a entrega digitando o número do pedido e selecionando o tier de taxa, e o backend persiste — vinculando ao iFood quando possível…
          </div>
        </div>

        {/* H2 — bullets */}
        <h2 style={{ fontSize: 21, fontWeight: 600, color: c.textStrong, margin: '24px 0 10px', position: 'relative' }}>
          <span style={{ position: 'absolute', left: -32, color: c.dimmer, fontFamily: mono, fontSize: 14, top: 5 }}>##</span>
          Próximos passos
        </h2>
        <ul style={{ paddingLeft: 0, listStyle: 'none', margin: 0 }}>
          {[
            ['☑', 'Fechar branch feat/focusNFe', true],
            ['☐', 'OWASP audit — top 10 / 2021', false],
            ['☐', 'Criar nota Fee Tier Matrix', false],
            ['☐', 'Validar fluxo offline (failsafe)', false],
          ].map(([m, t, done], i) => (
            <li key={i} style={{ display: 'flex', alignItems: 'baseline', gap: 10, padding: '4px 0', fontSize: 14.5, color: done ? c.dim : c.text, textDecoration: done ? 'line-through' : 'none' }}>
              <span style={{ color: done ? c.green : c.dim, fontFamily: mono, fontSize: 13 }}>{m}</span>
              <span>{t}</span>
            </li>
          ))}
        </ul>

        {/* Inline image attachment */}
        <div style={{ margin: '20px 0', border: `1px solid ${c.border}`, borderRadius: 6, overflow: 'hidden' }}>
          <div style={{ height: 180, background: `repeating-linear-gradient(45deg, ${c.panel2}, ${c.panel2} 8px, ${c.panel} 8px, ${c.panel} 16px)`, display: 'grid', placeItems: 'center', color: c.dim, fontFamily: mono, fontSize: 12 }}>
            ![[architecture-diagram.png]]
          </div>
          <div style={{ padding: '6px 12px', fontSize: 11, color: c.dimmer, fontFamily: mono, display: 'flex', alignItems: 'center', gap: 8, background: c.panel }}>
            <span>architecture-diagram.png</span>
            <span>·</span><span>1.2MB</span>
            <span style={{ marginLeft: 'auto', color: c.accent, cursor: 'pointer' }}>open</span>
          </div>
        </div>

        {/* Slash command — open inline */}
        <div style={{
          position: 'relative', marginTop: 20,
          fontSize: 14.5, color: c.text, fontFamily: sans,
        }}>
          <span style={{ color: c.dim }}>Comecei a esboçar a próxima discussão</span> <span style={{ color: c.accent, background: c.accentBg, padding: '1px 4px', borderRadius: 2 }}>/</span>
          {/* Slash popup */}
          <div style={{
            position: 'absolute', left: 232, top: 22, width: 280,
            background: c.panel2, border: `1px solid ${c.borderStrong}`, borderRadius: 6,
            boxShadow: `0 8px 24px rgba(0,0,0,0.4)`, zIndex: 5,
          }}>
            <div style={{ padding: '6px 12px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, borderBottom: `1px solid ${c.border}` }}>AI ACTIONS</div>
            {[
              ['✦', '/summarize', 'Resumir esta nota'],
              ['❓', '/ask', 'Perguntar ao vault'],
              ['✎', '/tag-auto', 'Auto-tag via LLM'],
            ].map(([g, cmd, desc], i) => (
              <div key={cmd} style={{
                display: 'grid', gridTemplateColumns: '20px 1fr auto', gap: 10, alignItems: 'center',
                padding: '7px 12px', background: i === 0 ? c.accentBg : 'transparent', cursor: 'pointer',
              }}>
                <span style={{ color: c.accent, fontFamily: mono }}>{g}</span>
                <div>
                  <div style={{ fontSize: 12.5, color: c.textStrong, fontFamily: mono }}>{cmd}</div>
                  <div style={{ fontSize: 11, color: c.dim }}>{desc}</div>
                </div>
                {i === 0 && <Kbd>↵</Kbd>}
              </div>
            ))}
            <div style={{ padding: '6px 12px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, borderTop: `1px solid ${c.border}`, borderBottom: `1px solid ${c.border}` }}>TEMPLATES</div>
            <div style={{ display: 'grid', gridTemplateColumns: '20px 1fr', gap: 10, padding: '7px 12px', alignItems: 'center', cursor: 'pointer' }}>
              <span style={{ color: c.dim, fontFamily: mono }}>▦</span>
              <div style={{ fontSize: 12.5, color: c.text, fontFamily: mono }}>/template meeting</div>
            </div>
            <div style={{ display: 'grid', gridTemplateColumns: '20px 1fr', gap: 10, padding: '7px 12px', alignItems: 'center', cursor: 'pointer' }}>
              <span style={{ color: c.dim, fontFamily: mono }}>▦</span>
              <div style={{ fontSize: 12.5, color: c.text, fontFamily: mono }}>/template daily</div>
            </div>
          </div>
        </div>

        <div style={{ height: 100 }}></div>
      </div>

      {/* Wikilink hover preview popup — floating */}
      <div style={{
        position: 'absolute', top: 270, left: 380, width: 340,
        background: c.panel, border: `1px solid ${c.borderStrong}`, borderRadius: 6,
        boxShadow: `0 12px 32px rgba(0,0,0,0.5)`, padding: '12px 14px', zIndex: 4,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
          <Icon size={11} color={c.blue}>{typeGlyph.resumo}</Icon>
          <span style={{ fontSize: 11, color: c.dim, fontFamily: mono }}>preview</span>
          <span style={{ marginLeft: 'auto', fontFamily: mono, fontSize: 10, color: c.dimmer }}>3 backlinks</span>
        </div>
        <div style={{ fontSize: 14, fontWeight: 600, color: c.textStrong, marginBottom: 4 }}>iFood Linking</div>
        <div style={{ fontSize: 12, color: c.dim, lineHeight: 1.5, marginBottom: 8 }}>
          Branch <span style={{ fontFamily: mono, color: c.amber }}>focusNFe</span>. Sistema de vinculação opcional de pedidos do app ao backend do iFood, com fallback para fluxo manual…
        </div>
        <div style={{ display: 'flex', gap: 6 }}>
          <HashTag>integration</HashTag>
          <HashTag>cfo-pocket</HashTag>
        </div>
      </div>
    </div>
  </div>
);

// ────────────── Right rail ──────────────

const RightRail = () => (
  <div style={{ width: 320, background: c.panel, borderLeft: `1px solid ${c.border}`, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
    {/* Tabs */}
    <div style={{ display: 'flex', borderBottom: `1px solid ${c.border}` }}>
      {[
        ['Backlinks', false, 7],
        ['Outline', false, 4],
        ['AI Chat', true],
      ].map(([t, active, count]) => (
        <button key={t} style={{
          flex: 1, padding: '10px 6px', border: 'none',
          background: active ? c.bg : 'transparent',
          color: active ? c.textStrong : c.dim,
          borderBottom: active ? `2px solid ${c.accent}` : '2px solid transparent',
          fontFamily: sans, fontSize: 12, fontWeight: active ? 600 : 500,
          cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6,
        }}>
          {t}
          {count && <span style={{ fontSize: 10, color: c.dimmer, fontFamily: mono }}>{count}</span>}
        </button>
      ))}
    </div>

    {/* AI Chat tab content */}
    <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      {/* Provider switcher */}
      <div style={{ padding: '8px 12px', borderBottom: `1px solid ${c.divider}`, display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ width: 6, height: 6, borderRadius: '50%', background: c.green }}></span>
        <span style={{ fontSize: 11, fontFamily: mono, color: c.dim }}>claude-sonnet-4.5</span>
        <span style={{ marginLeft: 'auto', fontSize: 10, color: c.dimmer }}>▾</span>
      </div>

      {/* Messages */}
      <div style={{ flex: 1, overflow: 'hidden', padding: '12px 12px 0' }}>
        {/* User msg */}
        <div style={{ marginBottom: 14, paddingLeft: 24, position: 'relative' }}>
          <div style={{ position: 'absolute', left: 0, top: 0, width: 18, height: 18, borderRadius: 4, background: c.panel2, display: 'grid', placeItems: 'center', fontSize: 10, color: c.dim }}>J</div>
          <div style={{ fontSize: 12.5, color: c.text, lineHeight: 1.55 }}>
            Como está o status do módulo Motoboys? Resumir os blockers.
          </div>
          <div style={{ display: 'flex', gap: 4, marginTop: 6 }}>
            <span style={{ fontSize: 10, fontFamily: mono, color: c.dim, border: `1px solid ${c.border}`, padding: '1px 6px', borderRadius: 3 }}>+ SPRINT.md</span>
            <span style={{ fontSize: 10, fontFamily: mono, color: c.dim, border: `1px solid ${c.border}`, padding: '1px 6px', borderRadius: 3 }}>+ Motoboys</span>
          </div>
        </div>

        {/* Assistant msg */}
        <div style={{ marginBottom: 14, paddingLeft: 24, position: 'relative' }}>
          <div style={{ position: 'absolute', left: 0, top: 0, width: 18, height: 18, borderRadius: 4, background: c.accentBg, border: `1px solid ${c.accent}`, display: 'grid', placeItems: 'center', fontSize: 10, color: c.accent, fontWeight: 700 }}>✦</div>
          <div style={{ fontSize: 12.5, color: c.text, lineHeight: 1.6 }}>
            Status: 95% no branch <InlineCode>focusNFe</InlineCode>. Três blockers ativos:
            <ol style={{ paddingLeft: 18, margin: '6px 0' }}>
              <li>OWASP audit pendente — ver <WikiLink>OWASP-2021 checklist</WikiLink></li>
              <li><WikiLink broken>Fee Tier Matrix</WikiLink> ainda não foi criada</li>
              <li>Failsafe offline não validado em campo</li>
            </ol>
            O fluxo do app Flutter está fechado.
          </div>
          {/* Sources */}
          <div style={{ marginTop: 8, padding: '6px 10px', background: c.inset, border: `1px solid ${c.border}`, borderRadius: 4 }}>
            <div style={{ fontSize: 10, color: c.dimmer, letterSpacing: 1, marginBottom: 4 }}>SOURCES · 4</div>
            <div style={{ fontSize: 11.5, color: c.accent, lineHeight: 1.7 }}>
              <div>↳ <WikiLink>Motoboys module</WikiLink></div>
              <div>↳ <WikiLink>SPRINT#review-2026-05-20</WikiLink></div>
              <div>↳ <WikiLink>iFood Linking</WikiLink></div>
              <div>↳ <WikiLink>HUMAN#login-flow</WikiLink></div>
            </div>
          </div>
        </div>
      </div>

      {/* Input */}
      <div style={{ borderTop: `1px solid ${c.border}`, padding: 10 }}>
        <div style={{
          background: c.inset, border: `1px solid ${c.border}`, borderRadius: 6,
          padding: '8px 10px',
        }}>
          <div style={{ display: 'flex', gap: 4, marginBottom: 8, flexWrap: 'wrap' }}>
            <span style={{ fontSize: 10, fontFamily: mono, color: c.accent, background: c.accentBg, padding: '1px 6px', borderRadius: 3, border: `1px solid ${c.accent}55` }}>2026-05-20.md ×</span>
            <span style={{ fontSize: 10, fontFamily: mono, color: c.dim, padding: '1px 6px', cursor: 'pointer' }}>+ attach</span>
          </div>
          <div style={{ color: c.dim, fontSize: 12.5 }}>Pergunte ao vault…</div>
          <div style={{ display: 'flex', alignItems: 'center', marginTop: 6, gap: 6 }}>
            <Icon size={12} color={c.dim}>📎</Icon>
            <Icon size={12} color={c.dim}>🎙</Icon>
            <span style={{ marginLeft: 'auto', display: 'flex', gap: 6, alignItems: 'center' }}>
              <Kbd>⌘↵</Kbd>
              <button style={{ background: c.accent, color: '#0a0a0a', border: 'none', borderRadius: 4, padding: '4px 12px', fontWeight: 700, fontSize: 11, cursor: 'pointer' }}>Send</button>
            </span>
          </div>
        </div>
      </div>
    </div>
  </div>
);

// ────────────── Chrome ──────────────

const TitleBar = () => (
  <div style={{ height: 36, background: c.chrome, borderBottom: `1px solid ${c.border}`, display: 'flex', alignItems: 'center', padding: '0 12px', gap: 10 }}>
    <div style={{ display: 'flex', gap: 6 }}>
      <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#ff5f56' }}></span>
      <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#ffbd2e' }}></span>
      <span style={{ width: 11, height: 11, borderRadius: '50%', background: '#27c93f' }}></span>
    </div>
    <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginLeft: 14 }}>
      <IconBtn>‹</IconBtn>
      <IconBtn>›</IconBtn>
    </div>
    <div style={{ flex: 1, textAlign: 'center', fontFamily: sans, fontSize: 12, color: c.dim }}>
      OmniNote <span style={{ color: c.dimmer }}>·</span> ClaudeBook <span style={{ color: c.dimmer }}>·</span> 2026-05-20.md
    </div>
    <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
      <span style={{ display: 'flex', alignItems: 'center', gap: 5, fontFamily: mono, fontSize: 11, color: c.red, padding: '2px 8px', border: `1px solid ${c.red}55`, borderRadius: 4, background: 'rgba(229,113,90,0.10)' }}>
        <span style={{ width: 6, height: 6, borderRadius: '50%', background: c.red, animation: 'pulse 1.5s infinite' }}></span>
        REC 00:34
      </span>
      <IconBtn>🎙</IconBtn>
      <IconBtn>⌘</IconBtn>
      <IconBtn>⚙</IconBtn>
    </div>
    <style>{`@keyframes pulse { 50% { opacity: 0.4; } }`}</style>
  </div>
);

const StatusBar = () => (
  <div style={{ height: 24, background: c.chrome, borderTop: `1px solid ${c.border}`, display: 'flex', alignItems: 'center', padding: '0 12px', gap: 14, fontFamily: mono, fontSize: 10.5, color: c.dimmer }}>
    <span style={{ color: c.green }}>● saved</span>
    <span>ln 47, col 12</span>
    <span>markdown</span>
    <span>UTF-8</span>
    <span>LF</span>
    <span style={{ marginLeft: 'auto' }}>vault sync: <span style={{ color: c.green }}>idle</span></span>
    <span>LLM: claude-sonnet-4.5</span>
    <span>v1.2.0</span>
  </div>
);

// ────────────── Toast ──────────────
const Toast = ({ kind, title, body }) => {
  const tone = { info: c.accent, ok: c.green, warn: c.amber, err: c.red }[kind] || c.accent;
  return (
    <div style={{
      background: c.panel, border: `1px solid ${c.border}`, borderLeft: `3px solid ${tone}`,
      borderRadius: 4, padding: '8px 12px', minWidth: 240, maxWidth: 320,
      boxShadow: `0 6px 20px rgba(0,0,0,0.4)`, fontFamily: sans,
    }}>
      <div style={{ display: 'flex', alignItems: 'baseline', gap: 6 }}>
        <span style={{ fontSize: 12, fontWeight: 600, color: c.textStrong }}>{title}</span>
        <span style={{ marginLeft: 'auto', color: c.dimmer, fontSize: 11, cursor: 'pointer' }}>×</span>
      </div>
      <div style={{ fontSize: 11.5, color: c.dim, marginTop: 2, lineHeight: 1.4 }}>{body}</div>
    </div>
  );
};

// ────────────── MAIN VIEW (artboard 1) ──────────────
const MainView = () => (
  <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: sans, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
    <TitleBar />
    <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
      <Sidebar />
      <Editor />
      <RightRail />
    </div>
    <StatusBar />

    {/* Toast stack */}
    <div style={{ position: 'absolute', bottom: 36, right: 16, display: 'flex', flexDirection: 'column', gap: 8 }}>
      <Toast kind="info" title="Note reloaded" body="2026-05-20.md was edited by Obsidian — re-read from disk." />
      <Toast kind="ok" title="OCR finished" body="paper.pdf · 12 pages · paper.ocr.md linked." />
    </div>
  </div>
);

// ────────────── COMMAND PALETTE (artboard 2) ──────────────
const CommandPalette = () => (
  <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: sans, display: 'flex', flexDirection: 'column', overflow: 'hidden', position: 'relative' }}>
    <TitleBar />
    <div style={{ flex: 1, display: 'flex', minHeight: 0, opacity: 0.5 }}>
      <Sidebar />
      <Editor />
      <RightRail />
    </div>
    <StatusBar />

    {/* Backdrop */}
    <div style={{ position: 'absolute', inset: '36px 0 24px 0', background: 'rgba(0,0,0,0.5)' }}></div>

    {/* Palette */}
    <div style={{
      position: 'absolute', top: 88, left: '50%', transform: 'translateX(-50%)',
      width: 620, background: c.panel, border: `1px solid ${c.borderStrong}`, borderRadius: 8,
      boxShadow: `0 24px 60px rgba(0,0,0,0.6)`, overflow: 'hidden',
    }}>
      {/* Input */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, padding: '14px 18px', borderBottom: `1px solid ${c.border}` }}>
        <Icon size={16} color={c.accent}>⌘</Icon>
        <span style={{ flex: 1, fontSize: 16, color: c.text }}>motob<span style={{ background: c.accent, color: c.chrome, padding: '0 1px', animation: 'blink 1s infinite' }}> </span></span>
        <Kbd>esc</Kbd>
      </div>

      {/* Scope chips */}
      <div style={{ display: 'flex', gap: 4, padding: '8px 18px', borderBottom: `1px solid ${c.divider}` }}>
        {[['All', true], ['Notes', false], ['Commands', false], ['Tags', false], ['Disciplines', false]].map(([t, active]) => (
          <span key={t} style={{
            padding: '2px 10px', fontSize: 11, fontFamily: sans,
            background: active ? c.accentBg : 'transparent', color: active ? c.accent : c.dim,
            border: `1px solid ${active ? c.accent : c.border}`, borderRadius: 999, cursor: 'pointer',
          }}>{t}</span>
        ))}
      </div>

      {/* Results */}
      <div style={{ padding: '8px 0' }}>
        <div style={{ padding: '4px 18px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, fontWeight: 600 }}>NOTES</div>
        {[
          [typeGlyph.resumo, typeColor.resumo, 'Motoboys module', 'Specs · 2m ago', true],
          [typeGlyph.codigo, typeColor.codigo, 'iFood Linking', 'cfo-pocket · 4h ago'],
          [typeGlyph.resumo, typeColor.resumo, 'Motoboy fee tiers', 'Drafts · 2d ago'],
        ].map(([g, col, title, meta, active], i) => (
          <div key={i} style={{
            display: 'grid', gridTemplateColumns: '24px 1fr auto', alignItems: 'center', gap: 12,
            padding: '8px 18px', background: active ? c.accentBg : 'transparent',
            borderLeft: active ? `2px solid ${c.accent}` : '2px solid transparent', cursor: 'pointer',
          }}>
            <Icon size={13} color={col}>{g}</Icon>
            <div>
              <div style={{ fontSize: 13, color: c.textStrong }}>
                <span style={{ color: c.accent }}>Motob</span>oys module
              </div>
              <div style={{ fontSize: 11, color: c.dim }}>{meta}</div>
            </div>
            {active && <Kbd>↵</Kbd>}
          </div>
        ))}

        <div style={{ padding: '8px 18px 4px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, fontWeight: 600 }}>COMMANDS</div>
        {[
          ['✦', 'AI · Resumir nota atual', '⌘⇧S'],
          ['+', 'Nova nota a partir de template', '⌘T'],
          ['🎙', 'Dictation · Toggle gravação', '⌘⇧M'],
          ['⊞', 'Toggle right rail', '⌘\\'],
        ].map(([g, t, k]) => (
          <div key={t} style={{
            display: 'grid', gridTemplateColumns: '24px 1fr auto', alignItems: 'center', gap: 12,
            padding: '8px 18px', cursor: 'pointer',
          }}>
            <Icon size={13} color={c.dim}>{g}</Icon>
            <div style={{ fontSize: 13, color: c.text }}>{t}</div>
            <Kbd>{k}</Kbd>
          </div>
        ))}

        <div style={{ padding: '8px 18px 4px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, fontWeight: 600 }}>TAGS</div>
        <div style={{ padding: '6px 18px 10px', display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          <HashTag>motoboy</HashTag>
          <HashTag>cfo-pocket</HashTag>
          <HashTag>sprint</HashTag>
        </div>
      </div>

      {/* Footer */}
      <div style={{ borderTop: `1px solid ${c.border}`, padding: '6px 18px', display: 'flex', alignItems: 'center', gap: 14, background: c.chrome }}>
        <span style={{ fontFamily: mono, fontSize: 10, color: c.dim, display: 'flex', alignItems: 'center', gap: 5 }}>
          <Kbd>↑↓</Kbd> navigate
        </span>
        <span style={{ fontFamily: mono, fontSize: 10, color: c.dim, display: 'flex', alignItems: 'center', gap: 5 }}>
          <Kbd>↵</Kbd> open
        </span>
        <span style={{ fontFamily: mono, fontSize: 10, color: c.dim, display: 'flex', alignItems: 'center', gap: 5 }}>
          <Kbd>⌘↵</Kbd> open in split
        </span>
        <span style={{ marginLeft: 'auto', fontSize: 10, color: c.dimmer }}>23 results in 18ms</span>
      </div>
    </div>
    <style>{`@keyframes blink { 50% { opacity: 0; } }`}</style>
  </div>
);

// ────────────── SPRINT VIEW (artboard 3) ──────────────
const SprintView = () => {
  const Task = ({ status, id, title, points, tag, owner, kind }) => {
    const sCol = { done: c.green, doing: c.amber, todo: c.dim, blocked: c.red }[status];
    return (
      <div style={{
        display: 'grid', gridTemplateColumns: '24px 60px 1fr auto auto auto', alignItems: 'center', gap: 12,
        padding: '10px 14px', borderBottom: `1px solid ${c.divider}`, cursor: 'pointer',
      }}>
        <span style={{ width: 14, height: 14, borderRadius: 3, border: `1.5px solid ${sCol}`, background: status === 'done' ? sCol : 'transparent', display: 'grid', placeItems: 'center', color: c.chrome, fontSize: 10 }}>
          {status === 'done' && '✓'}
        </span>
        <span style={{ fontFamily: mono, fontSize: 11, color: c.dim }}>{id}</span>
        <span style={{ fontSize: 13.5, color: status === 'done' ? c.dim : c.text, textDecoration: status === 'done' ? 'line-through' : 'none' }}>{title}</span>
        {kind && <span style={{ padding: '1px 7px', fontSize: 10, fontFamily: mono, color: typeColor[kind] || c.dim, border: `1px solid ${typeColor[kind] || c.border}55`, borderRadius: 3 }}>{kind}</span>}
        {tag && <HashTag>{tag}</HashTag>}
        <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontFamily: mono, fontSize: 11, color: c.dim }}>
          <span style={{ width: 18, height: 18, borderRadius: '50%', background: c.panel2, display: 'grid', placeItems: 'center', color: c.dim, fontSize: 9 }}>{owner}</span>
          {points}p
        </span>
      </div>
    );
  };

  return (
    <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: sans, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <TitleBar />
      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <Sidebar />
        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
          {/* Tab strip */}
          <div style={{ height: 32, background: c.panel2, borderBottom: `1px solid ${c.border}`, display: 'flex' }}>
            <Tab title="2026-05-20.md" kind="resumo" />
            <Tab title="SPRINT.md" kind="resumo" active />
            <div style={{ flex: 1 }}></div>
          </div>

          {/* Header */}
          <div style={{ padding: '24px 36px 16px', borderBottom: `1px solid ${c.divider}` }}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 14 }}>
              <Icon size={22} color={c.green}>◈</Icon>
              <div style={{ fontSize: 26, fontWeight: 700, color: c.textStrong, letterSpacing: -0.5 }}>Sprint 26 · cfo-pocket</div>
              <span style={{ padding: '3px 10px', fontSize: 11, color: c.green, background: `${c.green}22`, border: `1px solid ${c.green}66`, borderRadius: 999, fontFamily: mono }}>ACTIVE</span>
              <span style={{ marginLeft: 'auto', fontFamily: mono, fontSize: 11, color: c.dim }}>open SPRINT.md raw →</span>
            </div>
            <div style={{ color: c.dim, fontSize: 13, marginTop: 6 }}>
              May 12 → May 26 · <span style={{ color: c.text }}>day 9 of 14</span> · 47 pts committed · 32 done
            </div>

            {/* Progress bar */}
            <div style={{ marginTop: 16, height: 6, background: c.inset, borderRadius: 3, overflow: 'hidden', display: 'flex' }}>
              <div style={{ width: '68%', background: c.green }}></div>
              <div style={{ width: '12%', background: c.amber }}></div>
              <div style={{ width: '6%', background: c.red }}></div>
            </div>
            <div style={{ display: 'flex', gap: 16, marginTop: 8, fontFamily: mono, fontSize: 11, color: c.dim }}>
              <span><span style={{ color: c.green }}>● 32</span> done</span>
              <span><span style={{ color: c.amber }}>● 6</span> in progress</span>
              <span><span style={{ color: c.red }}>● 3</span> blocked</span>
              <span><span style={{ color: c.dim }}>● 6</span> todo</span>
            </div>
          </div>

          {/* Tasks */}
          <div style={{ flex: 1, overflow: 'hidden' }}>
            <div style={{ padding: '12px 14px 6px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, fontWeight: 600 }}>IN PROGRESS · 6</div>
            <Task status="doing" id="SCRUM-241" title="Motoboys module — finalizar OWASP audit" points={5} tag="motoboy" owner="J" kind="codigo" />
            <Task status="doing" id="SCRUM-238" title="Inline embeds no editor (markdown render)" points={8} tag="ui" owner="J" kind="codigo" />
            <Task status="blocked" id="SCRUM-235" title="Failsafe offline para fluxo de entrega" points={3} tag="motoboy" owner="M" />

            <div style={{ padding: '12px 14px 6px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, fontWeight: 600 }}>TODO · 6</div>
            <Task status="todo" id="SCRUM-244" title="Criar nota Fee Tier Matrix" points={2} owner="J" kind="definicao" />
            <Task status="todo" id="SCRUM-243" title="MCP tool descriptions para todos os verbos CLI" points={5} owner="J" />
            <Task status="todo" id="SCRUM-242" title="Calendar widget para daily notes" points={3} owner="J" kind="codigo" />

            <div style={{ padding: '12px 14px 6px', fontSize: 10, color: c.dimmer, letterSpacing: 1.5, fontWeight: 600 }}>DONE · 32 <span style={{ color: c.dimmer, fontWeight: 400, letterSpacing: 0 }}>(collapsed)</span></div>
            <Task status="done" id="SCRUM-237" title="iFood linking — branch focusNFe" points={8} owner="J" kind="codigo" />
            <Task status="done" id="SCRUM-234" title="Three-pane layout v1" points={5} owner="J" />
          </div>
        </div>
        <RightRail />
      </div>
      <StatusBar />
    </div>
  );
};

// ────────────── QUICK CAPTURE + Toast (artboard 4) ──────────────
const QuickCapture = () => (
  <div style={{ width: '100%', height: '100%', background: c.bg, color: c.text, fontFamily: sans, display: 'flex', flexDirection: 'column', overflow: 'hidden', position: 'relative' }}>
    <TitleBar />
    <div style={{ flex: 1, display: 'flex', minHeight: 0, opacity: 0.45 }}>
      <Sidebar />
      <Editor />
      <RightRail />
    </div>
    <StatusBar />

    {/* Quick capture popup — frameless, center-top */}
    <div style={{
      position: 'absolute', top: 64, left: '50%', transform: 'translateX(-50%)',
      width: 520, background: c.panel, border: `1px solid ${c.borderStrong}`, borderRadius: 10,
      boxShadow: `0 20px 60px rgba(0,0,0,0.6)`, overflow: 'hidden',
    }}>
      <div style={{ padding: '14px 18px', borderBottom: `1px solid ${c.border}`, display: 'flex', alignItems: 'center', gap: 10 }}>
        <Icon size={14} color={c.accent}>▼</Icon>
        <span style={{ fontFamily: sans, fontSize: 12, fontWeight: 600, color: c.textStrong }}>Quick capture</span>
        <span style={{ fontFamily: mono, fontSize: 10, color: c.dim }}>→ Inbox.md</span>
        <span style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
          <Kbd>⌘⇧Space</Kbd>
        </span>
      </div>
      <div style={{ padding: '14px 18px' }}>
        <div style={{
          fontSize: 16, color: c.text, fontFamily: sans, padding: '8px 0',
        }}>
          Lembrar: revisar a planilha de fee tiers com a Ju<span style={{ background: c.accent, padding: '0 1px', animation: 'blink 1s infinite' }}> </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 12 }}>
          <span style={{ fontSize: 10, color: c.dimmer, letterSpacing: 1.5, marginRight: 4 }}>TYPE</span>
          <TypeChip kind="resumo" active />
          <TypeChip kind="duvida" />
          <TypeChip kind="exercicio" />
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 14, paddingTop: 12, borderTop: `1px solid ${c.divider}` }}>
          <span style={{ fontSize: 11, color: c.dim, fontFamily: mono }}>+0 attachments</span>
          <span style={{ marginLeft: 'auto', display: 'flex', gap: 6, alignItems: 'center' }}>
            <Kbd>esc</Kbd>
            <span style={{ fontSize: 11, color: c.dim }}>cancel</span>
            <Kbd>↵</Kbd>
            <button style={{ background: c.accent, color: '#0a0a0a', border: 'none', borderRadius: 4, padding: '5px 14px', fontWeight: 700, fontSize: 11, cursor: 'pointer', fontFamily: sans }}>Capturar</button>
          </span>
        </div>
      </div>
    </div>

    {/* Dictation overlay — bottom-center */}
    <div style={{
      position: 'absolute', bottom: 56, left: '50%', transform: 'translateX(-50%)',
      width: 380, background: c.panel, border: `1px solid ${c.borderStrong}`, borderRadius: 10,
      boxShadow: `0 12px 36px rgba(0,0,0,0.5)`, padding: '12px 18px',
      display: 'flex', alignItems: 'center', gap: 14,
    }}>
      <span style={{ width: 12, height: 12, borderRadius: '50%', background: c.red, boxShadow: `0 0 12px ${c.red}`, animation: 'pulse 1.5s infinite' }}></span>
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 12, color: c.textStrong, fontWeight: 600 }}>Dictation · recording</div>
        <div style={{ display: 'flex', alignItems: 'end', gap: 2, marginTop: 6, height: 20 }}>
          {[0.3, 0.7, 0.5, 0.9, 0.6, 0.8, 0.4, 0.7, 0.3, 0.9, 0.5, 0.6, 0.8, 0.4, 0.7, 0.5, 0.3, 0.6, 0.9, 0.4].map((h, i) => (
            <span key={i} style={{ width: 3, height: `${h * 100}%`, background: c.accent, borderRadius: 1, opacity: 0.7 }}></span>
          ))}
        </div>
      </div>
      <span style={{ fontFamily: mono, fontSize: 12, color: c.dim }}>01:24</span>
      <Kbd>⌘⇧M</Kbd>
    </div>
    <style>{`@keyframes pulse { 50% { opacity: 0.4; } } @keyframes blink { 50% { opacity: 0; } }`}</style>
  </div>
);

window.MainView = MainView;
window.CommandPalette = CommandPalette;
window.SprintView = SprintView;
window.QuickCapture = QuickCapture;
