# Connect GitHub with a key file

This is the second way to connect GitHub to a hub: you make a GitHub App yourself at GitHub and give
the hub its key file. Use it on a hub that a browser cannot open: a hub that Anima.ing hosts (until
the gateway at `hub.anima.ing` exists, DEF-003 of [work-items](../../work-items/README.md)) and a hub
on this computer. On a hub with an address of its own, "Connect GitHub" does the same with three
presses (PRJ-039).

After the connection the hub is in the same state as after "Connect GitHub": both ways end in the
same step of the hub, which proves the key with one request to GitHub and then stores it (SRC-003,
PRJ-018). The differences are before that step:

- You choose the permissions of the App by hand. A permission that you leave out makes the function
  that needs it fail later, with a refusal of GitHub.
- The App has no address of the hub in it. Nothing at GitHub points to your hub.
- The App has the name that you give it.

Not verified: this procedure was not run on a hosted hub. It needs a hub that can reach
`api.github.com`; if the hub cannot, step 14 ends with a refusal that says that GitHub was not reached, and the hub stores nothing. The names of the permissions in step 7 are the names of GitHub for the 23 permissions of the manifest; they were not compared with the page of GitHub on the day of this text, so if a name differs, take the nearest one and say so.

## At GitHub

1. Open `https://github.com/settings/apps` (your picture, Settings, Developer settings, GitHub Apps).
   For an organisation: the settings of the organisation, Developer settings, GitHub Apps.
2. Press "New GitHub App".
3. "GitHub App name": a name that no App of GitHub has, for example `anima-` and your user name.
4. "Homepage URL": `https://anima.ing`. GitHub asks for an address; the hub does not use it.
5. Leave "Callback URL" empty. Do not tick "Request user authorization (OAuth) during installation".
6. Under "Webhook", remove the tick at "Active".
7. Under "Permissions", "Repository permissions", set each of these. They are the permissions that
   "Connect GitHub" asks for (K-E1 of [sources.md](../../../contracts/extensions/sources.md)):
   - "Metadata": Read-only (GitHub sets it by itself).
   - Read and write for: Actions, Administration, Checks, Code scanning alerts, Contents, Custom
     properties, Dependabot alerts, Dependabot secrets, Deployments, Discussions, Environments,
     Issues, Merge queues, Packages, Pages, Projects, Pull requests, Secret scanning alerts, Secrets,
     Variables, Webhooks, Workflows, and Commit statuses.
   Set no account permission and no organisation permission.
8. "Where can this GitHub App be installed?": "Only on this account".
9. Press "Create GitHub App". The page of the App shows "App ID": a number. Write it down.
10. On the same page, under "Private keys", press "Generate a private key". The browser saves a file
    whose name ends in `.private-key.pem`. This file is a secret: who has it can act as the App.
11. In the list at the left, press "Install App", then "Install" beside your account. Choose "All
    repositories" or the repositories that Anima may see, and press "Install".
12. The address of the page that follows ends in `/installations/` and a number. That number is the
    installation ID. Write it down.

## In Anima.ing

13. Open Settings, Integrations, GitHub, and press "Use key file".
14. Give a name (for example "Personal"), the App ID of step 9, the installation ID of step 12, and
    choose the `.pem` file of step 10. Press "Connect". Expected: the account shows in the list of
    GitHub accounts, and its state becomes healthy after its first read. If the hub refuses, it says
    why and stores nothing: check the two numbers, and that the file is the one of this App.
15. Delete the `.pem` file from the computer. The hub has the key now, sealed with the key of the
    hub; the application keeps no copy.

## To take the connection away

Remove the account in Settings, Integrations, GitHub. Then, at GitHub, open the App (step 1), and in
"Advanced" press "Delete GitHub App": the key stops working at once.
