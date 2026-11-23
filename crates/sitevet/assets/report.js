// sitevet report: theme switch, search, category filter and sort. Everything reads fine without it.
(() => {
  const d = document, root = d.documentElement;
  const all = (s) => [...d.querySelectorAll(s)];
  const store = (v) => { try { return v === undefined ? localStorage.getItem("sitevet-theme") : localStorage.setItem("sitevet-theme", v); } catch (e) { return null; } };

  const saved = store();
  if (saved) root.dataset.theme = saved;
  all(".js").forEach((e) => { e.hidden = false; });

  d.getElementById("theme").addEventListener("click", () => {
    const dark = root.dataset.theme ? root.dataset.theme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches;
    root.dataset.theme = dark ? "light" : "dark";
    store(root.dataset.theme);
  });

  // The selected page row follows the URL hash.
  const list = d.querySelector(".page-list");
  const rows = all(".page-row");
  const first = d.querySelector(".detail-page");
  const select = () => {
    const id = location.hash.slice(1) || (first && first.id);
    rows.forEach((a) => (a.hash.slice(1) === id ? a.setAttribute("aria-current", "true") : a.removeAttribute("aria-current")));
  };
  addEventListener("hashchange", select);
  select();

  // Page search and the category filter combine.
  let cat = "";
  const pageSearch = d.getElementById("page-search");
  const apply = () => {
    const term = pageSearch.value.trim().toLowerCase();
    rows.forEach((a) => {
      a.parentElement.hidden = !a.dataset.search.includes(term) || (cat !== "" && !a.dataset.cats.split(" ").includes(cat));
    });
    all(".detail .finding").forEach((f) => { f.hidden = cat !== "" && f.dataset.cat !== cat; });
  };
  pageSearch.addEventListener("input", apply);
  all(".cat").forEach((b) => b.addEventListener("click", () => {
    cat = b.dataset.cat;
    all(".cat").forEach((o) => o.setAttribute("aria-pressed", String(o === b)));
    apply();
  }));

  const catSearch = d.getElementById("cat-search");
  catSearch.addEventListener("input", () => {
    const term = catSearch.value.trim().toLowerCase();
    all(".cat").forEach((b) => { b.parentElement.hidden = b.dataset.cat !== "" && !b.textContent.toLowerCase().includes(term); });
  });

  // Sort: lowest score first (the order on load) or by path.
  all("[data-sort]").forEach((b) => b.addEventListener("click", () => {
    all("[data-sort]").forEach((o) => o.setAttribute("aria-pressed", String(o === b)));
    const items = rows.map((a) => a.parentElement);
    if (b.dataset.sort === "path") items.sort((x, y) => x.firstElementChild.dataset.path.localeCompare(y.firstElementChild.dataset.path));
    list.append(...items);
  }));

  // "/" focuses the page search, "c" the category search.
  addEventListener("keydown", (e) => {
    if (e.metaKey || e.ctrlKey || e.altKey || /^(INPUT|TEXTAREA|SELECT)$/.test(e.target.tagName)) return;
    const box = e.key === "/" ? pageSearch : e.key === "c" ? catSearch : null;
    if (!box || box.offsetParent === null) return;
    e.preventDefault();
    box.focus();
  });
})();
