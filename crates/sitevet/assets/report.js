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

  // Page details live as HTML strings in #pages-data. Only the open page is in the document,
  // which keeps a 100-page report at a few thousand elements.
  const pages = JSON.parse(d.getElementById("pages-data").textContent);
  const list = d.querySelector(".page-list");
  const rows = all(".page-row");
  const detail = d.querySelector(".detail");
  const detailEmpty = d.getElementById("detail-empty");
  const pagesEmpty = d.getElementById("pages-empty");
  const pageSearch = d.getElementById("page-search");
  let cat = "";
  let open = null;

  const filterFindings = () => all(".detail .finding").forEach((f) => { f.hidden = cat !== "" && f.dataset.cat !== cat; });
  const show = (id) => {
    if (id === open) return;
    open = id;
    const old = detail.querySelector(".detail-page");
    if (old) old.remove();
    if (id && pages[id]) detail.insertAdjacentHTML("beforeend", pages[id]);
    rows.forEach((a) => (a.hash.slice(1) === id ? a.setAttribute("aria-current", "true") : a.removeAttribute("aria-current")));
    filterFindings();
  };
  // The open page follows the URL hash, and falls back to the first row.
  const select = () => {
    const id = location.hash.slice(1);
    show(pages[id] ? id : rows.length ? rows[0].hash.slice(1) : null);
  };
  addEventListener("hashchange", select);
  select();

  // Page search and the category filter combine.
  const apply = () => {
    const term = pageSearch.value.trim().toLowerCase();
    rows.forEach((a) => {
      a.parentElement.hidden = !a.dataset.search.includes(term) || (cat !== "" && !a.dataset.cats.split(" ").includes(cat));
    });
    filterFindings();

    // Keep the detail on a page the list still shows, or say why the list is empty.
    const shown = rows.filter((a) => !a.parentElement.hidden);
    if (shown.length && !shown.some((a) => a.hash.slice(1) === open)) location.replace(shown[0].hash);
    if (!shown.length) show(null);
    else if (open === null) select();
    detailEmpty.hidden = pagesEmpty.hidden = shown.length > 0;
    if (shown.length) return;
    const name = cat && d.querySelector(`.cat[data-cat="${cat}"] .cat-name`).textContent;
    const site = cat ? all(`.view-site .finding[data-cat="${cat}"]`).length : 0;
    pagesEmpty.textContent = term ? "No page matches this search." : `No page has ${name} issues.`;
    if (!term && site) {
      const link = d.createElement("label");
      link.htmlFor = "v-site";
      link.className = "link";
      link.textContent = `${site} site-wide ${site === 1 ? "finding" : "findings"}`;
      pagesEmpty.append(" See the ", link, ".");
    }
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
