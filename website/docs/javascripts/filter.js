/* Filters the tables of an index page. The field `input.botica-filter` hides the rows that do not
   hold every word typed in it, or in the heading of their section, and the sections (a heading
   and what follows it up to the next heading of that level) that have no row left. `?q=` in the
   address carries the words, so a filtered list can be linked. `scripts/site_filter.py` puts the
   field in the page. */
(function () {
  "use strict";

  function words(query) {
    return query.toLowerCase().split(/\s+/).filter(Boolean);
  }

  function sectionsOf(article, rows) {
    var sections = [];
    article.querySelectorAll("h2").forEach(function (heading) {
      var elements = [heading];
      for (var next = heading.nextElementSibling; next && next.tagName !== "H2"; next = next.nextElementSibling) {
        elements.push(next);
      }
      var held = [];
      rows.forEach(function (row, index) {
        if (elements.some(function (element) { return element.contains(row); })) {
          held.push(index);
        }
      });
      if (held.length) {
        sections.push({ elements: elements, rows: held });
      }
    });
    return sections;
  }

  function setup(input) {
    if (input.dataset.bound) {
      return;
    }
    input.dataset.bound = "true";
    var article = input.closest("article") || document.body;
    var count = article.querySelector(".botica-filter-count");
    var rows = Array.prototype.slice.call(article.querySelectorAll("tbody tr"));
    var haystack = rows.map(function (row) { return row.textContent.toLowerCase(); });
    var sections = sectionsOf(article, rows);
    /* A word of the heading of a section, such as the family of a rule, matches its rows. */
    sections.forEach(function (section) {
      var heading = section.elements[0].textContent.toLowerCase();
      section.rows.forEach(function (index) { haystack[index] += " " + heading; });
    });

    function apply() {
      var typed = words(input.value);
      var shown = 0;
      rows.forEach(function (row, index) {
        var match = typed.every(function (word) { return haystack[index].indexOf(word) !== -1; });
        row.hidden = !match;
        if (match) {
          shown += 1;
        }
      });
      sections.forEach(function (section) {
        var any = section.rows.some(function (index) { return !rows[index].hidden; });
        section.elements.forEach(function (element) { element.hidden = !any; });
      });
      if (count) {
        if (!typed.length) {
          count.textContent = rows.length + (rows.length === 1 ? " entry" : " entries");
        } else if (!shown) {
          count.textContent = "No entry matches";
        } else {
          count.textContent = shown + " of " + rows.length + " shown";
        }
      }
    }

    function remember() {
      var query = input.value.trim();
      var address = query ? "?q=" + encodeURIComponent(query) : location.pathname;
      history.replaceState(history.state, "", address + location.hash);
    }

    input.addEventListener("input", function () {
      apply();
      remember();
    });
    input.value = new URLSearchParams(location.search).get("q") || "";
    apply();
  }

  function init() {
    document.querySelectorAll("input.botica-filter").forEach(setup);
  }

  if (typeof document$ !== "undefined" && document$.subscribe) {
    document$.subscribe(init);
  } else {
    document.addEventListener("DOMContentLoaded", init);
  }
})();
