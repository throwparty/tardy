# Tardy - Meeting Bar Foundation

**Status:** draft  **Created:** 2026-08-07  **Author:** Luke Carrier

Tardy is a Linux analogue of the Meeting Bar app usually seen on macOS. It keeps sight of the user's upcoming meetings and offers a single control to jump straight into whichever meeting service hosts a given meeting. Tardy presents two views of the same surface. A persistent overlay sits above the user's other windows so the schedule is never missed. A compact one-line summary sits in the desktop's status area for at-a-glance awareness. The user's meetings come from the calendar accounts they have already registered on the machine, so Tardy works with whatever calendar setup the desktop already knows about rather than requiring the user to configure or authorise anything new inside the app. A meeting's join control always opens the correct service for that meeting, no matter which provider is hosting it.


## Problem

The user frequently runs late to meetings because there is no lightweight at-a-glance view of the day's meetings on their desktop with one-click join

## Goals

- Surface the user's upcoming meetings for a configurable lookahead window
- Provide a persistent overlay above all windows with a per-meeting join control
- Keep the user informed without any per-app account setup; the machine's existing calendars are used as-is

## Non-goals

- This ADR covers the foundation only, not packed distribution or packaging


## Non-goals

- No modification of the user's calendars; Tardy only reads them


## Non-goals

- No in-app login or credential entry; only calendars the desktop already knows are surfaced


## Functional Requirements

### FR-001: Show Upcoming Meetings

The app displays the user's meetings from the calendar accounts available on the machine, restricted to a configurable lookahead window

**Slug:** `show-upcoming-meetings`

### FR-002: Configurable Lookahead

The user can configure how many days ahead meetings are shown, defaulting to 5 days

**Slug:** `lookahead-days`

### FR-003: Height Over Other Windows

A persistent on-screen panel shows the meetings and stays above all other windows on every attached display

**Slug:** `overlay-above`

### FR-004: Join A Meeting

Each meeting offers a control that launches the meeting, opening the correct service regardless of the conference provider

**Slug:** `join-meeting`

### FR-005: Text Status Summary

The app maintains a short text summary of the upcoming meetings for display in the desktop status area

**Slug:** `status-summary`

### FR-006: Automatic Refresh

Upcoming meetings refresh automatically so that the panel promptly reflects new, changed or started meetings, including a meeting approaching its start time

**Slug:** `automatic-refresh`

### FR-007: Meeting Without A Link

A meeting that offers no usable join link is still shown, with its join control visibly disabled

**Slug:** `no-link`

## Non-functional Requirements

### NFR-001: Provider Agnostic

Joining works for the meeting services the user's calendars reference, not only for one specific vendor

**Slug:** `provider-agnostic`

### NFR-002: Runs Without New Auth

The app runs using the calendar credentials the desktop already holds; it surfaces what it can even when one account's calendar cannot be reached and it never demand that the user enter a fresh sign-in solely for Tardy

**Slug:** `no-new-auth`

### NFR-003: Resilient To Missing Data

If a calendar account cannot be read the app stays up, continues to show the calendars it can, and recovers once the blocked account is available

**Slug:** `resilient`

## Acceptance Criteria

### AC-001: Shows Registered Calendars

With a desktop-confirmed Google calendar, the app enumerates and shows that calendar's meetings for the lookahead window

**Slug:** `shows-calendars`

### AC-002: Join Opens Service

Activating a meeting's join control launches the meeting's own service such that the user lands in the meeting

**Slug:** `join-opens-service`

### AC-003: Overlay Raised Amply

The panel is visible above all other windows on each attached display without any interleaving window action

**Slug:** `overlay-raised`

### AC-004: Lookahead Configurable

Changing the configured lookahead changes the set of meetings surfaced

**Slug:** `lookahead-configurable`

### AC-005: Unavailable Account Safe

With a calendar account that cannot be refreshed, the app stays functional, shows an explicit empty state, and recovers when the account is back

**Slug:** `unavailable-account-safe`

## Edge Cases

### EC-001: No Calendars Available

The panel and the status summary both show a clear empty state and remain ready to update

**Slug:** `no-calendars`

## Edge Cases

### EC-002: Meeting Without A Link

If a meeting cannot yield a usable join link its join control is disabled but the meeting is still shown

**Slug:** `meeting-no-link`

## Edge Cases

### EC-003: Account Drops Mid-Refresh

When a calendar account becomes unavailable at refresh time the partial list from the other calendars is still shown and the account is retried

**Slug:** `account-drop`

