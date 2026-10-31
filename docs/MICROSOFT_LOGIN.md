# Microsoft sign-in setup

Salt signs in with Microsoft's **device code flow**: the user types a short
code on microsoft.com in their own browser. Salt never sees a password.

To do that, Salt needs an **Azure application (client) ID** that Mojang has
approved for the Minecraft API. This is a one-time setup for whoever ships
Salt; end users never see it once it's baked into a build.

## 1. Register an Azure app

1. Go to <https://portal.azure.com> → **App registrations** → **New registration**.
2. Name: `saltClient`. Supported account types: **Personal Microsoft accounts only**.
3. Redirect URI: leave empty (device code flow doesn't need one).
4. After creating it, open **Authentication** → set **Allow public client flows** to **Yes** → Save.
5. Copy the **Application (client) ID** (a GUID) from the Overview page.

## 2. Ask Mojang to approve it

Since 2022, new Azure apps get `403 Invalid app registration` from
`api.minecraftservices.com` until Mojang approves them. Apply with the form at
<https://aka.ms/mce-reviewappid> (it asks for the client ID and a short
description of the launcher). Approval can take a while.

Until it's approved, sign-in gets through Microsoft and Xbox and then fails
at the Minecraft step with a clear error in the launcher log.

## 3. Use the ID

* **For your own testing:** paste it into Settings → Integrations → Client ID
  (or the onboarding sign-in step).
* **For a release build:** bake it in, so users never have to:

  ```powershell
  $env:SALT_MS_CLIENT_ID = "your-client-id-guid"
  cargo build --release -p salt-launcher
  ```

The client ID is public information (it ships inside every launcher binary);
it is not a secret and doesn't need protecting.
