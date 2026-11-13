// Huy hiệu đánh số trên mockup (giải thích nằm trong UI-CONCEPT.md). Thêm #clean vào URL để ẩn.
function showNotes(list) {
  if (location.hash === '#clean') return;
  list.forEach(({ n, x, y, t }) => {
    const d = document.createElement('div');
    d.className = 'callout';
    d.style.left = x + 'px'; d.style.top = y + 'px';
    d.title = t || '';
    d.textContent = n;
    document.body.appendChild(d);
  });
}
