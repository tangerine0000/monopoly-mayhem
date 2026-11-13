// Dữ liệu 40 ô lấy từ crates/monopoly-core/src/board.rs (tên + giá thật)
const TILES = [
  { n: 'Khởi hành', k: 'go' },
  { n: 'Bến Vân Đồn', d: 'q4', p: 600 },
  { n: 'Khí vận', k: 'chest' },
  { n: 'Đoàn Văn Bơ', d: 'q4', p: 600 },
  { n: 'Thuế thu nhập', k: 'tax', p: 2000 },
  { n: 'BX Miền Đông', k: 'bus', p: 2000 },
  { n: 'Phạm Thế Hiển', d: 'q8', p: 1000 },
  { n: 'Cơ hội', k: 'chance' },
  { n: 'Tạ Quang Bửu', d: 'q8', p: 1000 },
  { n: 'Dương Bá Trạc', d: 'q8', p: 1200 },
  { n: 'Tạm giam', k: 'jail' },
  { n: 'Phan Xích Long', d: 'pn', p: 1400 },
  { n: 'Cấp nước', k: 'water', p: 1500 },
  { n: 'Phan Đăng Lưu', d: 'pn', p: 1400 },
  { n: 'Hoàng Văn Thụ', d: 'pn', p: 1600 },
  { n: 'Ga Sài Gòn', k: 'train', p: 2000 },
  { n: 'Cộng Hòa', d: 'tb', p: 1800 },
  { n: 'Khí vận', k: 'chest' },
  { n: 'Trường Chinh', d: 'tb', p: 1800 },
  { n: 'Lê Văn Sỹ', d: 'tb', p: 2000 },
  { n: 'Cà phê bệt', k: 'park' },
  { n: 'Hải Thượng Lãn Ông', d: 'q5', p: 2200 },
  { n: 'Cơ hội', k: 'chance' },
  { n: 'Châu Văn Liêm', d: 'q5', p: 2200 },
  { n: 'An Dương Vương', d: 'q5', p: 2400 },
  { n: 'BX Miền Tây', k: 'bus', p: 2000 },
  { n: 'Nguyễn Thị Minh Khai', d: 'q3', p: 2600 },
  { n: 'Nam Kỳ Khởi Nghĩa', d: 'q3', p: 2600 },
  { n: 'Điện lực', k: 'bolt', p: 1500 },
  { n: 'Võ Văn Tần', d: 'q3', p: 2800 },
  { n: 'Vào tạm giam', k: 'gojail' },
  { n: 'Trần Não', d: 'q2', p: 3000 },
  { n: 'Song Hành', d: 'q2', p: 3000 },
  { n: 'Khí vận', k: 'chest' },
  { n: 'Xuân Thủy', d: 'q2', p: 3200 },
  { n: 'Metro số 1', k: 'train', p: 2000 },
  { n: 'Cơ hội', k: 'chance' },
  { n: 'Đồng Khởi', d: 'q1', p: 3500 },
  { n: 'Thuế xa xỉ', k: 'tax', p: 1000 },
  { n: 'Nguyễn Huệ', d: 'q1', p: 4000 },
];

const ICON = {
  go: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 12H5M11 6l-6 6 6 6"/></svg>',
  chest: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><rect x="5" y="3" width="14" height="18" rx="2" fill="#c8372d" stroke="#c8372d"/><circle cx="12" cy="10" r="3.2" stroke="#f2c14e" stroke-width="1.6"/><path d="M8 16h8" stroke="#f2c14e"/></svg>',
  chance: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M3 7a2 2 0 0 0 2-2h14a2 2 0 0 0 2 2v10a2 2 0 0 0-2 2H5a2 2 0 0 0-2-2z" fill="#f2c14e" stroke="#b5801c"/><path d="M10 10a2 2 0 1 1 2.8 1.8c-.6.3-.8.7-.8 1.2M12 15.5v.2" stroke="#2a1712" stroke-width="2" stroke-linecap="round"/></svg>',
  tax: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M6 3h12v18l-3-2-3 2-3-2-3 2z"/><path d="M9 8h6M9 12h6M9 16h3"/></svg>',
  bus: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><rect x="4" y="4" width="16" height="13" rx="3"/><path d="M4 11h16M7 20v-3M17 20v-3"/><circle cx="8" cy="14" r=".8" fill="currentColor"/><circle cx="16" cy="14" r=".8" fill="currentColor"/></svg>',
  train: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><rect x="6" y="3" width="12" height="14" rx="4"/><path d="M6 10h12M9 21l2-4M15 21l-2-4"/><circle cx="9.5" cy="13.5" r=".9" fill="currentColor"/><circle cx="14.5" cy="13.5" r=".9" fill="currentColor"/></svg>',
  water: '<svg viewBox="0 0 24 24" fill="none" stroke="#2b7c8c" stroke-width="1.8"><path d="M12 3s6 7 6 11a6 6 0 0 1-12 0c0-4 6-11 6-11z" fill="#2b7c8c22"/></svg>',
  bolt: '<svg viewBox="0 0 24 24" fill="#e3a72f" stroke="#b5801c" stroke-width="1.4" stroke-linejoin="round"><path d="M13 2 4 14h7l-1 8 9-12h-7z"/></svg>',
  jail: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8"><rect x="4" y="4" width="16" height="16" rx="2"/><path d="M8.5 4v16M12 4v16M15.5 4v16"/></svg>',
  park: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M5 9h11v5a5 5 0 0 1-5 5h-1a5 5 0 0 1-5-5z"/><path d="M16 11h1.5a2.5 2.5 0 0 1 0 5H16M8 3v3M11 3v3M14 3v3"/></svg>',
  gojail: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round"><path d="M6 18a6 6 0 0 1 12 0z" fill="#c8372d33" stroke="#c8372d"/><path d="M12 4v4M5 7l2 2M19 7l-2 2M4 21h16"/></svg>',
};

// Phương tiện của người chơi (thay cho T1..T4)
const VEHICLE = {
  cub: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="16" r="3"/><circle cx="18" cy="16" r="3"/><path d="M6 16h4l3-6h3l2 6M13 10l-1-3h-2"/></svg>',
  xichlo: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="5" cy="17" r="2.5"/><circle cx="12" cy="17" r="2.5"/><circle cx="19" cy="17" r="2.5"/><path d="M3 8h8v5H3zM11 13l8 4M16 9l3 8M15 9h3"/></svg>',
  vespa: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="6" cy="17" r="2.5"/><circle cx="18" cy="17" r="2.5"/><path d="M4 14c0-3 3-5 7-5h2l3 8H8.5M15 9l-1-4h3"/></svg>',
  bus: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="3" y="5" width="18" height="11" rx="2.5"/><path d="M3 10h18"/><circle cx="7.5" cy="18" r="1.8"/><circle cx="16.5" cy="18" r="1.8"/></svg>',
};

function cell(i) {
  if (i === 0) return [11, 11];
  if (i < 10) return [11 - i, 11];
  if (i === 10) return [1, 11];
  if (i < 20) return [1, 11 - (i - 10)];
  if (i === 20) return [1, 1];
  if (i < 30) return [1 + (i - 20), 1];
  if (i === 30) return [11, 1];
  return [11, 1 + (i - 30)];
}

function fmtK(k) { // 2200 (nghìn) -> "2,2tr" ; 600 -> "600k"
  return k >= 1000 ? (k / 1000).toLocaleString('vi-VN', { maximumFractionDigits: 1 }) + 'tr' : k + 'k';
}

/**
 * opts: { owners: {idx: 'p1'}, houses: {idx: n}, mortgaged: [idx], landing: idx, trail: [idx], dim: bool,
 *         tokens: [{seat:'p1', vehicle:'cub', at: idx, active: true}], compact: bool }
 */
function renderBoard(el, opts = {}) {
  const o = Object.assign({ owners: {}, houses: {}, mortgaged: [], trail: [], tokens: [] }, opts);
  TILES.forEach((t, i) => {
    const [c, r] = cell(i);
    const tile = document.createElement('div');
    const corner = i % 10 === 0;
    const side = !corner && (c === 1 || c === 11);
    tile.className = 'tile' + (corner ? ' corner' : '') + (side ? ' side ' + (c === 1 ? 'left' : 'right') : '') + (r === 1 && !corner ? ' top-row' : '');
    tile.style.gridColumn = c; tile.style.gridRow = r;
    tile.dataset.idx = i;
    const owner = o.owners[i];
    if (owner) { tile.classList.add('owned'); tile.style.setProperty('--owner', `var(--${owner})`); }
    if (o.mortgaged.includes(i)) tile.classList.add('mortgaged');
    if (o.landing === i) tile.classList.add('landing');
    if (o.trail.includes(i)) tile.classList.add('trail');

    let plate = '';
    if (t.d) {
      const h = o.houses[i] || 0;
      const marks = h === 5 ? '<span class="hotel"></span>' : '<span class="house"></span>'.repeat(h);
      plate = `<div class="plate" style="background:var(--${t.d})">${marks}</div>`;
    }
    const icon = t.k ? `<span class="icon" style="${corner ? 'width:34px;height:34px' : ''}">${ICON[t.k]}</span>` : '';
    const price = t.p && !t.k?.match(/tax/) ? `<span class="price num">${fmtK(t.p)}</span>` : (t.k === 'tax' ? `<span class="price num">-${fmtK(t.p)}</span>` : (t.k === 'go' ? '<span class="price">+2tr</span>' : ''));
    const name = o.compact && !t.d && !corner ? '' : `<span class="name">${t.n}</span>`;
    tile.innerHTML = `${plate}<div class="body">${icon}${name}${price}</div>${owner ? `<span class="flag">${owner.slice(1)}</span>` : ''}`;
    el.appendChild(tile);
  });

  requestAnimationFrame(() => placeTokens(el, o.tokens));
}

function placeTokens(el, tokens) {
  const bySpot = {};
  tokens.forEach((tk) => {
    const tile = el.querySelector(`.tile[data-idx="${tk.at}"]`);
    if (!tile) return;
    const n = (bySpot[tk.at] = (bySpot[tk.at] || 0) + 1) - 1;
    // Quân đứng ở mép trong của ô (phía giữa bàn), không che tên đường
    const [c, r] = cell(tk.at);
    const L = tile.offsetLeft, T = tile.offsetTop, w = tile.offsetWidth, h = tile.offsetHeight;
    let x, y;
    if (tk.at % 10 === 0) { x = L + w * 0.22 + n * 16; y = T + h * 0.5; }
    else if (r === 11) { x = L + w / 2 + n * 16; y = T + 2; }            // hàng dưới: đứng trên mép trên
    else if (r === 1) { x = L + w / 2 + n * 16; y = T + h + 40; }        // hàng trên: đứng dưới mép dưới
    else if (c === 1) { x = L + w + 22 + n * 16; y = T + h * 0.8; }      // cột trái
    else { x = L - 22 - n * 16; y = T + h * 0.8; }                        // cột phải
    const d = document.createElement('div');
    d.className = 'token' + (tk.active ? ' active' : '');
    d.style.left = x + 'px'; d.style.top = y + 'px';
    d.style.setProperty('--c', `var(--${tk.seat})`);
    d.innerHTML = `<div class="disc">${VEHICLE[tk.vehicle]}</div>`;
    el.appendChild(d);
  });
}

// Bản đồ trung tâm: sông Sài Gòn + họa tiết gạch bông
function centerMap(opacity = 1) {
  return `<svg class="map" viewBox="0 0 900 900" preserveAspectRatio="xMidYMid slice" style="opacity:${opacity}">
    <defs>
      <pattern id="gb" width="90" height="90" patternUnits="userSpaceOnUse">
        <rect width="90" height="90" fill="#efe3c9"/>
        <g fill="none" stroke="#d8c39b" stroke-width="2">
          <circle cx="0" cy="0" r="26"/><circle cx="90" cy="0" r="26"/><circle cx="0" cy="90" r="26"/><circle cx="90" cy="90" r="26"/>
          <path d="M45 18 L72 45 L45 72 L18 45Z"/><circle cx="45" cy="45" r="8" fill="#e6d3ad"/>
        </g>
      </pattern>
    </defs>
    <rect width="900" height="900" fill="url(#gb)"/>
    <path d="M-20 140 C 180 120, 260 300, 420 330 S 700 260, 760 430 S 700 760, 920 820" fill="none" stroke="#2b7c8c" stroke-opacity=".28" stroke-width="70" stroke-linecap="round"/>
    <path d="M-20 140 C 180 120, 260 300, 420 330 S 700 260, 760 430 S 700 760, 920 820" fill="none" stroke="#fff" stroke-opacity=".35" stroke-width="3" stroke-dasharray="2 14"/>
  </svg>`;
}
