[Plan index](../nosey-implementation-plan.md) · [Previous: 12. Pending changes and operations](12-pending-changes-and-operations.md) · [Next: 14. Registry Studio and publishing](14-registry-studio-and-publishing.md)

## 13. Desktop and onboarding

### 13.1 Navigation and views

```text
Projects
  └── Selected project
      ├── Overview
      ├── Scan
      ├── Recommendations
      └── Installed Skills
Explore
Library
Presets
Registries
  └── Registry Studio
Pending Changes
Settings
```

Retain existing Kitter visual language, theme capabilities, reusable GPUI components and other compatible features. The exact layout may incorporate existing instruction/configuration screens uncovered by the audit. Do not remove features merely to match this illustrative navigation.

| View | Requirements |
|---|---|
| Projects | Manual add, list, select, unregister with unlink choice, inaccessible paths |
| Overview | Project path, last scan status, detected counts, effective skills and pending changes |
| Scan | Manual scan/rescan/cancel, progress, detailed technology inventory, versions, rules, structured evidence, incomplete/stale/offline diagnostics, unmapped technologies |
| Recommendations | Missing, Managed, Unmanaged, Dismissed groups; mapping reasons; combination/priority order; target coverage; individual/bulk/install-all; removal candidates and confirmation |
| Installed Skills | Global/project effective availability, managed/unmanaged, target locations, unlink managed links only, token estimates |
| Explore | Complete registry browser, fuzzy search and filters, references, safe rendered skill preview |
| Library | Shared skill inventory, updates, source status, global/project usage, cleanup |
| Presets | Linked preset editing and pending synchronization |
| Registries | Global registry enable/order/trust/refresh, status, removal/cleanup, Studio launch |
| Pending Changes | Cross-project/global selected change review and application, result/retry |
| Settings | Global targets, custom relative paths, scan limits/exclusions, registry TTL, application/skill update modes, YAML settings import/export, onboarding restart |

All asynchronous I/O, parsing and scanning run off the main UI thread. Display loading, empty, error, disabled, stale and partial states. Keep action selection stable when lists are refreshed or filtered, unless the selected operation becomes invalid. Never show an apparent installed state before verifying successful execution.

### 13.2 Optional wizard

First-launch wizard can be skipped and reopened from Settings. Cover official registry status and initialization, global agent selection, optional custom targets, manual first project registration, explicit first scan and optional approval of recommended installations. Skipping onboarding leaves valid defaults and an empty Projects screen. No Kitter migration step.

### 13.3 Notifications and activity

Use in-app notifications, badges/status indicators, pending counts and contextual failure messages only. Recent activity includes timestamps, affected project/scope, operation type and result for scans, install/unlink, updates, registry refreshes, publishing and synchronization. Bounded retention (proposed 100 recent entries; confirm during implementation). No native Linux notification system, system tray, telemetry or background service. A dedicated permanent notification audit center is not a requirement.

---

[Plan index](../nosey-implementation-plan.md) · [Previous: 12. Pending changes and operations](12-pending-changes-and-operations.md) · [Next: 14. Registry Studio and publishing](14-registry-studio-and-publishing.md)
