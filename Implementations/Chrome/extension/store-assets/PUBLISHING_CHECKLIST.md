# Publishing checklist — Chrome Web Store InputKey 5.2.13

Existing item ID: `bogegcgnbkkpdnambaajjeceoiijdkjb`
Publisher: RentAICoder; use only the publisher Google account `vmquangvu@gmail.com`.
Update the existing published item, not a new listing.

- [x] Version 5.2.13 built using `Bootstrap/Build/build-chrome.ps1`; browser DOM tests pass 16/16, including English collision protection.
- [x] Extension metadata and both listing languages describe the five supported language packs and the “Type at the speed of thought” positioning.
- [x] Official website: https://vuminhquang.github.io/InputKey/
- [ ] Confirm new privacy page is live: https://vuminhquang.github.io/InputKey/privacy.html
- [ ] Upload exact vetted ZIP `dist/chrome/InputKey-5.2.13.zip` as a Package update to the existing item ID.
- [ ] Update the English/Vietnamese descriptions from `STORE_LISTING_EN.md` and `STORE_LISTING_VI.md`, not stale copy.
- [ ] Set Homepage URL to https://vuminhquang.github.io/InputKey/; Support URL to https://github.com/vuminhquang/InputKey/issues.
- [ ] Update Privacy policy URL; site access/permission justification and single purpose must cover all five supported languages.
- [ ] Update Test instructions from `REVIEW_NOTES.md`: English preservation, Vietnamese Telex/VNI and French/Danish/Swedish/German Telex.
- [ ] Confirm the existing icon and screenshots remain valid and descriptive; do not claim new features absent from the package.
- [ ] Save draft, inspect all tabs and submit for review; capture actual dashboard status. “Submitted” is not “Published.”
- [ ] Verify published version later on the public Store page; do not report success based only on local ZIP or source push.

Privacy: No typed-content collection/transmission; local settings in `chrome.storage.local`; packaged local WASM; no analytics/ads.
