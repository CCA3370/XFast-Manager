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

## Upgrading from a shared database

The old database and settings are kept but are not automatically assigned to
a root without reliable ownership information. Rebuild the scenery index once
for each installation. The initial rebuild reads the selected installation's
existing INI without writing it. Export any presets to keep before upgrading
and import them while their intended installation is selected. Old activity
history is not shown in the new installation histories. On first launch, XFast
carries enabled add-on beta preferences forward to the previously selected
installation once. Any preferences already saved for that installation take
precedence. Check that another installation has its own preferences even
after restarting with it selected. Re-enter any old unscoped application-only
locks, CSL paths or launch arguments that are still needed. Installed add-on
files are not migrated.
