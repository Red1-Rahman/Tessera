// Tessera
// Copyright (C) 2026 Redwan Rahman
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

import { expect, test } from "@playwright/test";

test("loads the application shell", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Tessera" })).toBeVisible();
});
