// Insert the rytmyk sun mark + chord wordmark at the top of the mdBook sidebar.
(function () {
  var MARK =
    '<svg viewBox="0 0 128 128" width="28" height="28" aria-hidden="true">' +
    '<defs><radialGradient id="rb-glow" cx="50%" cy="50%" r="50%">' +
    '<stop offset="0%" stop-color="#ff9d5c"/><stop offset="100%" stop-color="#ff9d5c" stop-opacity="0"/>' +
    '</radialGradient></defs>' +
    '<circle cx="64" cy="64" r="60" fill="url(#rb-glow)" opacity=".7"/>' +
    '<g fill="none" stroke="#ff9d5c" stroke-width="3">' +
    '<circle cx="64" cy="64" r="60" opacity=".25"/><circle cx="64" cy="64" r="47" opacity=".40"/>' +
    '<circle cx="64" cy="64" r="35" opacity=".60"/><circle cx="64" cy="64" r="24" opacity=".85"/></g>' +
    '<circle cx="64" cy="64" r="16" fill="#ff9d5c"/></svg>';

  function insertBrand() {
    // Insert INTO the scrollbox (which is absolutely positioned filling the
    // sidebar) so the brand sits in the scroll flow above the chapter list —
    // inserting it as a sibling would overlap the list at top:0.
    var box = document.querySelector('.sidebar-scrollbox');
    if (!box || document.getElementById('rytmyk-brand')) return;
    var a = document.createElement('a');
    a.id = 'rytmyk-brand';
    a.href = 'introduction.html';
    a.innerHTML = MARK + '<span>chord<span class="seed">.</span></span>';
    box.insertBefore(a, box.firstChild);
  }

  // Trim the theme picker to the two brand themes, relabelled (like the landing).
  // mdBook 0.5 ids: mdbook-theme-{default_theme,light,rust,coal,navy,ayu}.
  var KEEP = { 'mdbook-theme-light': 'Warm sand', 'mdbook-theme-navy': 'Dusk' };
  function tuneThemeMenu() {
    var btns = document.querySelectorAll('button.theme');
    if (!btns.length) return;
    btns.forEach(function (btn) {
      var li = btn.closest('li') || btn.parentElement;
      if (KEEP[btn.id]) { btn.textContent = KEEP[btn.id]; }
      else if (li) { li.style.display = 'none'; }   // hide Auto / Rust / Coal / Ayu
    });
  }

  function run() { insertBrand(); tuneThemeMenu(); }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', run);
  } else {
    run();
  }
  setTimeout(run, 50);
})();
