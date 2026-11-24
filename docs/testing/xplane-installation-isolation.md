# Verify changing X-Plane installations

Use two test installations, A and B, with backed-up `scenery_packs.ini` files.
Give them one same-named scenery package and at least one different package each.
Start with different load orders and enabled states. Keep X-Plane closed for
write/restore tests.

1. Select A in Settings and create its scenery index. Record the entries,
   order, enabled states and `scenery_packs.ini` contents. Select B and open
   Scenery. It can prompt to create its own index, but must not report A's
   packages as deleted or B's as added. Create B's index and verify the existing
   B load order and enabled states. Neither file should have changed.
2. Add or remove a test scenery package within B, then reopen Scenery. Confirm
   that the change notice concerns B only. Repeat with A after switching back.
   Also try creating an index for an empty Custom Scenery folder, then adding
   one package: only that folder should count as newly added.
3. Lock a same-named package in A and stage an enabled/order edit. Switch to B
   without applying. B must not inherit the lock or the staged edit. Make
   and apply a different change in B and verify A's INI stayed unchanged.
4. Repeat A → B → A with a scan or add-on update underway. The previous scan
   must not show an old list or alert after the switch. Old update progress
   must not attach to a same-named task, and a previously open confirmation
   must not act on B. If a write was already submitted for A, it must only
   write to A.
5. Verify Aircraft, Plugins, Navdata, Flight Plans, Screenshots, Scripts,
   Output Cleanup, Disk Usage, Health, Gateway, Airport Flatten and CSL show
   only the selected installation's data. Create a preset and activity entry
   in A and verify neither appears in B. Verify a CSL custom path, per-add-on
   beta preference and launch argument saved in A are not applied in B.
   Restart and switch back to A to verify that its saved values persist.
6. Disable a plugin in A to auto-lock it. In B, manually lock its same-named
   copy and enable that copy. The manual lock in B should stay in place. Return
   to A and enable its copy to clear its automatic lock. Also start applying a
   preset with a saved lock state in A, then switch to B before it finishes.
   Both the preset and its locks should finish applying in A. Verify that B's
   lock state and files are unchanged.
7. Switch roots with an aircraft patch preview or scenery rebuild in progress.
   The old patch preview should close. The new root should allow its own
   rebuild and should show its own results when it finishes.

## Upgrading from a shared database

XFast keeps the old database and settings. Gateway installations and Airport
Flatten overrides with an identifiable root carry over to that installation.
To verify, upgrade test installations with different Gateway packages and
flatten overrides. Check each list and confirm the packages and overrides still
belong to the expected installation. Remove a test override, restart, and
verify that the removal persists. Rebuild the scenery index once for each
installation. The initial rebuild reads the selected installation's existing
INI without writing it. Export any presets to keep before upgrading and
import them while their intended installation is selected. New activity
histories start empty. On first launch, XFast
carries enabled add-on beta preferences forward to the previously selected
installation once. Any preferences already saved for that installation take
precedence. Check that another installation has its own preferences even
after restarting with it selected. Re-enter any old unscoped application-only
locks, CSL paths or launch arguments that are still needed. Installed add-on
files are not migrated.

## Verify SkunkCrafts beta recovery

Open the update drawer for a legacy plugin that has both Stable and Beta. Load
a Stable plan, then make the Beta depot unavailable using a local test server.
Switch to Beta and verify that the drawer shows the failure while keeping the
channel toggle available. Turn Beta off in the same drawer and verify that the
Stable plan loads again. In a local HTTP fixture, blacklist a directory that
contains an obsolete file and add a new whitelisted file under that directory.
Also put a modified local file under a directory named in the add-on's local
ignore list. Run the update and verify that it removes the obsolete file,
keeps the newly downloaded file, and preserves the local modification.
