#!/usr/bin/env node
/**
 * One-shot probe: fetch a Moodle `mod/url` page and extract the external resource.
 *
 * Unauthenticated requests 303 to HKU Portal login and do not include the resource.
 * Pass a MoodleSession cookie to probe a logged-in page:
 *
 *   node scripts/probe-moodle-url.mjs 'https://moodle.hku.hk/mod/url/view.php?id=4229276'
 *   MOODLE_SESSION=... node scripts/probe-moodle-url.mjs <url>
 */

const TARGET =
  process.argv[2] ??
  "https://moodle.hku.hk/mod/url/view.php?id=4229276";

const UA =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

function absUrl(href, base) {
  try {
    return new URL(href, base).href;
  } catch {
    return href;
  }
}

function firstMatch(html, re) {
  const m = html.match(re);
  return m ? m[1] : null;
}

function extractFromHtml(html, finalUrl) {
  const title = firstMatch(html, /<title>([\s\S]*?)<\/title>/i);
  const heading = firstMatch(
    html,
    /<h2[^>]*>\s*([\s\S]*?)\s*<\/h2>/i,
  );
  const description = firstMatch(
    html,
    /<div class="activity-description"[^>]*id="intro"[\s\S]*?<div class="no-overflow">([\s\S]*?)<\/div>/i,
  );
  const workaroundHref = firstMatch(
    html,
    /<div class="urlworkaround">[\s\S]*?<a[^>]+href="([^"]+)"/i,
  );
  const iframeSrc = firstMatch(
    html,
    /<iframe[^>]+src="([^"]+)"/i,
  );
  const googleUrls = [
    ...html.matchAll(/https:\/\/docs\.google\.com\/[^\s"'<>]+/g),
  ].map((m) => m[0]);

  const bodyClass = firstMatch(html, /<body[^>]*class="([^"]*)"/i) ?? "";
  const needsLogin =
    /login\/index\.php/i.test(finalUrl) ||
    /Log in to the site/i.test(title ?? "") ||
    /HKU Portal user login/i.test(html);

  return {
    finalUrl,
    title: title?.replace(/\s+/g, " ").trim() ?? null,
    heading: heading?.replace(/<[^>]+>/g, "").replace(/\s+/g, " ").trim() ?? null,
    description: description
      ?.replace(/<[^>]+>/g, " ")
      .replace(/\s+/g, " ")
      .trim() ?? null,
    needsLogin,
    cmid: firstMatch(bodyClass, /cmid-(\d+)/),
    courseId: firstMatch(bodyClass, /course-(\d+)/),
    cmType: firstMatch(bodyClass, /cm-type-(\S+)/),
    urlworkaround: workaroundHref ? absUrl(workaroundHref, finalUrl) : null,
    iframeSrc: iframeSrc ? absUrl(iframeSrc, finalUrl) : null,
    googleUrls: [...new Set(googleUrls)],
    externalUrl: workaroundHref
      ? absUrl(workaroundHref, finalUrl)
      : iframeSrc
        ? absUrl(iframeSrc, finalUrl)
        : (googleUrls[0] ?? null),
  };
}

async function fetchHop(url, cookieHeader) {
  const headers = {
    Accept: "text/html,application/xhtml+xml",
    "Accept-Language": "en-US,en;q=0.9",
    "User-Agent": UA,
  };
  if (cookieHeader) headers.Cookie = cookieHeader;

  const res = await fetch(url, {
    headers,
    redirect: "manual",
  });
  const location = res.headers.get("location");
  const body = await res.text();
  return {
    url,
    status: res.status,
    location: location ? absUrl(location, url) : null,
    body,
  };
}

async function probe(startUrl) {
  const session = process.env.MOODLE_SESSION;
  const cookieHeader = session ? `MoodleSession=${session}` : undefined;
  const hops = [];
  let current = startUrl;
  let html = "";
  let finalUrl = startUrl;

  for (let i = 0; i < 8; i++) {
    const hop = await fetchHop(current, cookieHeader);
    hops.push({
      url: hop.url,
      status: hop.status,
      location: hop.location,
    });
    if (hop.location && hop.status >= 300 && hop.status < 400) {
      current = hop.location;
      continue;
    }
    html = hop.body;
    finalUrl = hop.url;
    break;
  }

  return {
    startUrl,
    authenticated: Boolean(cookieHeader),
    hops,
    ...extractFromHtml(html, finalUrl),
  };
}

const report = await probe(TARGET);
console.log(JSON.stringify(report, null, 2));

if (report.needsLogin) {
  console.error(
    "\nLogin wall: HKU Moodle 303s to /login/index.php (HKU Portal CAS). Re-run with MOODLE_SESSION set.",
  );
  process.exitCode = 2;
} else if (!report.externalUrl) {
  console.error("\nAuthenticated page loaded, but no external URL was found.");
  process.exitCode = 1;
}
