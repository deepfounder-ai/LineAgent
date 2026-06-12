#!/usr/bin/env python3
"""
Import Linear Drayinsight Cycle #2 (2026-06-08 → 2026-06-22) into LineAgent.

Usage:
    export LINEAGENT_API_URL=http://localhost:3000
    export LINEAGENT_API_KEY=lineagent_...
    python3 scripts/import_dra_cycle2.py
"""

import os
import sys
import json
import urllib.request
import urllib.error

BASE_URL = os.environ.get("LINEAGENT_API_URL", "http://localhost:3000").rstrip("/")
API_KEY  = os.environ.get("LINEAGENT_API_KEY", "")

if not API_KEY:
    print("ERROR: LINEAGENT_API_KEY not set", file=sys.stderr)
    sys.exit(1)


def req(method, path, body=None):
    url = BASE_URL + path
    data = json.dumps(body).encode() if body is not None else None
    r = urllib.request.Request(
        url, data=data, method=method,
        headers={
            "Authorization": f"Bearer {API_KEY}",
            "Content-Type": "application/json",
        },
    )
    try:
        with urllib.request.urlopen(r) as resp:
            raw = resp.read()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        if e.code == 404 and method == "GET":
            return None
        if e.code == 409:
            return {"_conflict": True, "raw": raw}
        print(f"  HTTP {e.code} {method} {path}: {raw[:200]}", file=sys.stderr)
        raise


def ensure_project(key, name):
    existing = req("GET", f"/api/v1/projects/{key}")
    if existing:
        print(f"  project {key} already exists")
        return existing
    result = req("POST", "/api/v1/projects", {"key": key, "name": name})
    print(f"  created project {key}")
    return result


def create_cycle(project_key, name, starts_at, ends_at):
    result = req("POST", f"/api/v1/projects/{project_key}/cycles",
                 {"name": name, "starts_at": starts_at, "ends_at": ends_at})
    if result and result.get("_conflict"):
        print(f"  cycle '{name}' may already exist, skipping")
        return None
    print(f"  created cycle: {name} ({result.get('id', '?')})")
    return result


def create_ticket(body):
    result = req("POST", "/api/v1/tickets", body)
    if result and result.get("_conflict"):
        print(f"    conflict on ticket '{body['title'][:50]}', skipping")
        return None
    return result


# ---------------------------------------------------------------------------
# Data
# ---------------------------------------------------------------------------

ISSUES = [
    # --- URGENT ---
    {
        "linear_id": "DRA-49",
        "title": "[Security] RFQ API: auth bypass + cross-tenant data access & destruction",
        "description": "Full-API security audit of the RFQ Rust/Axum engine (`rfq-api`). Three findings with the same root cause: there is no verified tenant identity — auth is a single shared key (or any Bearer string), and tenant scoping is taken from attacker-controlled request params. Two items confirmed exploitable live on prod.",
        "priority": "critical",
        "status": "in_progress",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-53",
        "title": "Stripe Subscription Desync — User Locked Out, Duplicate Subscription Created on Manual Fix",
        "description": "A paying customer (Jay, Accurate Transport) was unable to log in — the app reported the account didn't exist and prompted him to sign up. The subscription record in Bubble showed as 'not active' despite an active Stripe subscription. Manually toggling it to active triggered a new subscription to be created, leaving two records.",
        "priority": "critical",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    # --- HIGH ---
    {
        "linear_id": "DRA-36",
        "title": "QA Review (2026-05-22) — 39 bugs found across the app",
        "description": "Full UI/QA Review. Date: 2026-05-22. Tester: Claude (senior tester mode, Chrome automation). App: https://app.drayinsight.com. Single rollup issue for an end-to-end QA pass across Quotes, Opportunities, Lane Analysis, Customers, RFQ, and the full Company Settings tree.",
        "priority": "high",
        "status": "todo",
        "assignee": "Dmitrii",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-57",
        "title": "Quote save issues: stalls, missing data, wrong title displayed",
        "description": "Multiple issues when saving quotes: 1) Save stalls or takes very long — had to reload the page. 2) Title not showing in Dashboard after save. 3) Destination missing on one quote. 4) Wrong title displayed — another quote's title appeared after saving the next one. See Loom recording for reproduction.",
        "priority": "high",
        "status": "done",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-66",
        "title": "[Architecture] Route ALL quote creation through the Rust engine (Bubble calls the endpoint)",
        "description": "Make the Rust engine the single creator of quotes. Today quotes are created by two independent paths: 1) Rust engine (Excel import, widget lead, batch, calculate) — writes Supabase + Bubble. 2) Bubble UI 'Create Quote' workflow — writes Bubble directly. Two writers against the same data is the root of a whole class of bugs.",
        "priority": "high",
        "status": "review",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-63",
        "title": "Extension QA: 8 Issues — Margin, Terminal, Contact, PDF, Email",
        "description": "QA session surfaced 8 issues across margin calculation, terminal routing, contact auto-fill, PDF output, and email behavior. Approach: migrated Chrome extension backend from n8n webhooks → Supabase edge functions, fixing bugs 2–8 at the root.",
        "priority": "high",
        "status": "in_progress",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-64",
        "title": "[Bug] RFQ engine bug-hunt fixes (resume race, calc, geocoding panics)",
        "description": "Parallel multi-agent bug audit of the RFQ Rust engine. 7 confirmed bugs shipped in commits 90c0fb1 and 77d61c6; full suite 25 tests pass. Fixed: resume_task race → duplicate quotes, calc errors, geocoding panics.",
        "priority": "high",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-12",
        "title": "Hubspot integration — Customers sync",
        "description": "",
        "priority": "high",
        "status": "review",
        "assignee": "Dmitrii",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-60",
        "title": "Accepted quote not sent to TMSEZ — no Postmark email",
        "description": "Scheduled test with Terrier: accepted quote should be sent to TMSEZ. Quote is not showing in TMSEZ and no emails appear in Postmark logs. Steps: 1) Set up scheduled test with Terrier. 2) Accept quote. Expected: quote sent to TMSEZ via email (Postmark). Actual: quote missing from TMSEZ, no email in Postmark.",
        "priority": "high",
        "status": "todo",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-37",
        "title": "Refactor Login & Registration Flow — Performance",
        "description": "The current login and registration flow is noticeably slow. Goals: profile and identify the bottleneck, reduce end-to-end login time to under 1 second, reduce registration time to under 2 seconds, ensure no regression in security or multi-tenant isolation.",
        "priority": "high",
        "status": "todo",
        "assignee": "Dmitrii",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-28",
        "title": "Add field whitelist to bulk_update_quote_details + terminal checks to bulk_quote_move_to_*",
        "description": "Security/correctness issues in bulk mutation functions. 1) bulk_update_quote_details uses SET q = jsonb_populate_record without a field whitelist — any caller can overwrite any column. 2) bulk_quote_move_to_terminal and bulk_quote_move_to_opportunity neither verify that the terminal/opportunity belongs to the caller's tenant.",
        "priority": "high",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-40",
        "title": "Website Widget — IMC Test: 5 Issues Found",
        "description": "QA test of the website widget for IMC surfaced 5 issues. Terminal routing to Headquarters worked correctly when no terminal was matched, but several UX and data issues need fixing: saved origin location not prominent, customer field shows individual name, requestor details not in Contact Email, Transfer popup has no search, Needs Review status not highlighted.",
        "priority": "high",
        "status": "todo",
        "assignee": "Vladimir Alushkin",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-72",
        "title": "Quick Quote Fails with City/State Input — Requires Saved Location",
        "description": "Quick Quote only works when a saved location is selected. Entering a plain city/state address fails. Example: Calhoun → Savannah — saved location title is 'Savannah', not the full address. Entering 'Savannah, GA' directly does not resolve correctly; must select the saved location 'Savannah'.",
        "priority": "high",
        "status": "todo",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-70",
        "title": "[Bug] Flat-rate truck markup drops real maintenance/lease cost from quote cost",
        "description": "When a rate band's truck markup mode is 'Flat rate', the calculator dropped the truck's real operating cost (maintenance + lease/depreciation) from the quote. total_cost was understated and margin overstated. Reported on the rail-ramp → Indianapolis P2P lanes.",
        "priority": "high",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-69",
        "title": "[Bug] Estimate prices on default mileage profile while save uses P2P — price drift for typed saved locations",
        "description": "For a lane whose origin/destination is a saved location the user typed as a raw address, the estimate preview and the saved quote priced on different rate profiles → different linehaul, fuel, total, and margin for the same lane. Reported alongside DRA-68 on the same rail-ramp → Indianapolis P2P lanes.",
        "priority": "high",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-68",
        "title": "[Bug] P2P quotes saved from estimates leave Bubble trip_pricing empty (option-set casing)",
        "description": "Saving an estimate whose lane resolves to a point-to-point (P2P) rate profile created a quote whose Bubble trip_pricing was completely empty — no rate profile, no mileage/duration, no pricing. Supabase had correct values; only the Bubble side was blank. Mileage-based profiles were unaffected.",
        "priority": "high",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    # --- MEDIUM ---
    {
        "linear_id": "DRA-67",
        "title": "Add wait time at location field to locations table",
        "description": "Add a wait_time_minutes field to each saved location so users can see the average wait time at every location. Scope: 1) Supabase migration — ALTER TABLE public.saved_locations ADD COLUMN IF NOT EXISTS wait_time_minutes integer. 2) Expose in CreateLocationIframe. 3) Display in UI.",
        "priority": "medium",
        "status": "review",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-74",
        "title": "[SEO/Agent] Make website AI-agent ready (robots.txt, sitemap, discovery, MCP)",
        "description": "Audit from isitagentready.com — site missing all standard agent/crawler discovery infrastructure. Critical items: robots.txt not found, sitemap.xml not found, llms.txt not found, MCP endpoint missing. Must return 200 with correct content types.",
        "priority": "medium",
        "status": "todo",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-58",
        "title": "XLSX export in Opportunity uses PDF export settings",
        "description": "XLSX export in the Opportunity module is using PDF export settings instead of its own configuration. Export output may have incorrect formatting, layout, or data selection inherited from the PDF export path. Expected: XLSX export should use its own dedicated settings independent of PDF export configuration.",
        "priority": "medium",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-73",
        "title": "Prompt User to Recalculate When PC Miler-Dependent Fields Change",
        "description": "When fields that affect PC Miler distance/routing are changed (origin, destination, stops, route type, etc.), show a notification prompting the user to recalculate. Expected: 1) User edits a PC Miler-dependent field on a quote. 2) Notification appears: 'Fields affecting mileage have changed. Recalculate?' 3) User can confirm → triggers recalculation, or dismiss → keeps existing mileage.",
        "priority": "medium",
        "status": "todo",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-71",
        "title": "Quick-quote chips fail for city/state lanes (only saved-location chips worked)",
        "description": "Clicking a latest-lane quick-select chip whose origin/destination is a plain City, ST (not a saved location) produced an estimate the RFQ engine couldn't price. Chips backed by a saved location worked. Latest-lane chips backed by a plain city/state were not being resolved through the saved-locations lookup.",
        "priority": "medium",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-56",
        "title": "Reconfigure Bubble Supabase plugin to use authenticated JWT instead of anon key",
        "description": "Long-term follow-up to DRA-24 regression. DRA-24 attempted to revoke EXECUTE from anon on dangerous SECURITY DEFINER functions. The Bubble Supabase plugin uses the anon key for all DB calls, which means it bypasses RLS. Reconfigure to use an authenticated service-role JWT or a dedicated scoped key.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-44",
        "title": "Surface Saved Origin Locations at Top of Origin Selector",
        "description": "Cleveland, OH is a Terminal with a saved origin location, but it is not visible at the top of the origin selector in the widget. Saved locations should be surfaced first so prospective customers can easily pick the correct origin and route to the correct Terminal. Expected: saved origin locations appear at the top of the origin dropdown — above freeform search results.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Vladimir Alushkin",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-47",
        "title": "Customer Field Shows Individual Name Instead of Company Name",
        "description": "The Customer field in the widget displays the individual's name (e.g. 'Jon Sinton') instead of the company name (e.g. 'Dray Insight'). Expected: the Customer field should display the company name, not the individual user's name.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Vladimir Alushkin",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-48",
        "title": "Populate Requestor Details in 'Contact Email' Line of Quote",
        "description": "Name, email, and phone number submitted via the widget are not being added to the Contact Email line of the quote. Expected: the requestor's name, email, and phone number from the widget submission are populated in the Contact Email line of the quote. Do not automatically add the contact to the Customer section.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Vladimir Alushkin",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-45",
        "title": "Add Terminal Search to Transfer Popup Bar",
        "description": "The popup bar used to transfer a request to a Terminal has no search functionality. Users cannot type to find a terminal by name — they must scroll through the full list. Expected: a searchable text input is present in the transfer popup, filtering the terminal list as the user types.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Dmitrii",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-46",
        "title": "Highlight 'Needs Review' Status with Color",
        "description": "The 'Needs Review' status is not visually highlighted with a color. All statuses should be displayed with their corresponding color pill/badge — same as other status labels in the widget. Expected: 'Needs Review' status is styled with a distinct highlight color, consistent with other status indicators.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    {
        "linear_id": "DRA-32",
        "title": "Harden or remove locations_insert_anon RLS policy (WITH CHECK (true))",
        "description": "Security hardening. The locations_insert_anon RLS policy uses WITH CHECK (true), allowing any anonymous user to insert rows into the locations table without any validation. Options: 1) Add meaningful constraints: WITH CHECK (created_by IS NULL AND source = 'anonymous'). 2) Remove the policy entirely if anon inserts are no longer needed. 3) Rate-limit via a SECURITY DEFINER wrapper function.",
        "priority": "medium",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    # --- LOW ---
    {
        "linear_id": "DRA-51",
        "title": "[Improvement] RFQ API contract & error-handling cleanup",
        "description": "Contract/error-handling inconsistencies found during the full-endpoint live smoke test (prod). Non-critical but cause confusing behavior and minor info leaks. L1: POST /rfq/quotes/calculate persists despite its name — named 'calculate' (implies dry-run/preview) but it creates a full persistent quote in Supabase + Bubble.",
        "priority": "low",
        "status": "done",
        "assignee": None,
        "parent_linear_id": None,
    },
    # --- NO PRIORITY ---
    {
        "linear_id": "DRA-14",
        "title": "Customer request from IMC",
        "description": "Microsoft 365 integration",
        "priority": "medium",
        "status": "todo",
        "assignee": None,
        "parent_linear_id": None,
    },
    # --- PARENT for DRA-16, DRA-18 ---
    {
        "linear_id": "DRA-11",
        "title": "Rate Profile migration",
        "description": "",
        "priority": "high",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": None,
    },
    # --- CHILDREN ---
    {
        "linear_id": "DRA-16",
        "title": "rate profiles UI",
        "description": "",
        "priority": "high",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": "DRA-11",
    },
    {
        "linear_id": "DRA-18",
        "title": "Link Point 2 Point RP to Mileage based",
        "description": "",
        "priority": "high",
        "status": "todo",
        "assignee": "Kir Leshkevich",
        "parent_linear_id": "DRA-11",
    },
]

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    print(f"Connecting to LineAgent at {BASE_URL}")

    # 1. Ensure project DRA exists
    print("\n[1] Project")
    ensure_project("DRA", "Drayinsight")

    # 2. Create cycle
    print("\n[2] Cycle")
    cycle = create_cycle("DRA", "Cycle 2 (2026-06-08 → 2026-06-22)",
                         "2026-06-08T04:00:00Z", "2026-06-22T04:00:00Z")
    cycle_id = cycle["id"] if cycle else None

    # 3. Create tickets — roots first, then children
    print(f"\n[3] Tickets ({len(ISSUES)} total)")
    la_id_map = {}  # linear_id → lineagent_identifier

    roots = [i for i in ISSUES if i["parent_linear_id"] is None]
    children = [i for i in ISSUES if i["parent_linear_id"] is not None]

    for issue in roots + children:
        body = {
            "project_key": "DRA",
            "title": issue["title"],
            "status": issue["status"],
            "priority": issue["priority"],
        }
        if issue["description"]:
            body["description"] = issue["description"]
        if issue["assignee"]:
            body["assignee"] = issue["assignee"]
        if cycle_id:
            body["cycle_id"] = cycle_id

        # resolve parent
        if issue["parent_linear_id"]:
            parent_la = la_id_map.get(issue["parent_linear_id"])
            if parent_la:
                body["parent_identifier"] = parent_la
            else:
                print(f"  WARNING: parent {issue['parent_linear_id']} not yet imported, skipping parent link for {issue['linear_id']}")

        result = create_ticket(body)
        if result and result.get("identifier"):
            ident = result["identifier"]
            la_id_map[issue["linear_id"]] = ident
            print(f"  {issue['linear_id']} → {ident}  [{issue['status']}]  {issue['title'][:60]}")
        else:
            print(f"  {issue['linear_id']} SKIPPED/FAILED")

    print(f"\nDone. {len(la_id_map)}/{len(ISSUES)} tickets imported.")
    print("\nMapping:")
    for lin, la in sorted(la_id_map.items()):
        print(f"  {lin} → {la}")


if __name__ == "__main__":
    main()
