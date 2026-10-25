# Changelog

## [0.1.1](https://github.com/sejori/actualised-ai/compare/v0.1.0...v0.1.1) (2026-10-25)


### Features

* add send_message tool and webhook loop timeout warning ([bfdb5f9](https://github.com/sejori/actualised-ai/commit/bfdb5f90fcec5845ec8e7f982099deac83031c74))
* agent editing, continuous inference, tool library and dropzone ([905ebe3](https://github.com/sejori/actualised-ai/commit/905ebe38b881e21b36b6b50ab4461e02da6b310f))
* agent hierarchy swapping and UI updates ([8e56e8a](https://github.com/sejori/actualised-ai/commit/8e56e8a3c0a282e505503bc35579b6d09d6cc0cc))
* **ci:** switch npm release to tokenless OIDC Trusted Publishing ([780ef8e](https://github.com/sejori/actualised-ai/commit/780ef8e04e96cd514cc50546e0ed126d555e07e2))
* **ci:** switch npm release to tokenless OIDC Trusted Publishing ([f929f42](https://github.com/sejori/actualised-ai/commit/f929f421122f7c277644c074afdd4a9429eb61c6))
* **core:** add Doubleword provider support and per-agent model/temperature routing ([e5e7da8](https://github.com/sejori/actualised-ai/commit/e5e7da87481461a71120088b30bbc606f24ecfec))
* **core:** Doubleword AI provider & per-agent model/temperature routing ([1bff92e](https://github.com/sejori/actualised-ai/commit/1bff92e316bb92dd93ed0c44367ec5cf51c66c8d))
* **core:** Doubleword JSONL batch API engine & autonomous completion poller ([2a1c46c](https://github.com/sejori/actualised-ai/commit/2a1c46c207cb19bcb7aa269b416932360c600194))
* **core:** Doubleword JSONL batch API engine, agent tools, and completion turn poller ([c54d104](https://github.com/sejori/actualised-ai/commit/c54d104c6c90187f300f2d85acc5b806b2d35c49))
* implement bring-your-own-bot telegram architecture and backend settings persistence ([ee6e80f](https://github.com/sejori/actualised-ai/commit/ee6e80fb1373743055b84722f2c393122406c426))
* implement github webhook endpoints and sync issues to db cache ([7fdb27d](https://github.com/sejori/actualised-ai/commit/7fdb27dfa6d9b382a86dea3e9d32cf9767013f40))
* implement self-developing multi-agent architecture and cloud run infrastructure ([082f07b](https://github.com/sejori/actualised-ai/commit/082f07be834c43489b90fd19e04935dc79bb88b1))
* implement telegram notify tool, telegram webhook, and read_memory tool with prompt cache optimization ([38764ff](https://github.com/sejori/actualised-ai/commit/38764ff6a7cf3f56de722b2d59cd291c48db1912))
* multi-company sidebar and switching logic ([8307390](https://github.com/sejori/actualised-ai/commit/830739073f1bb9f92b0910e0426694aaee9b03e1))
* multi-tenancy authentication UI and backend routing ([97d302e](https://github.com/sejori/actualised-ai/commit/97d302e7758efd373a32691fec0672f670ed7c07))
* run companies continuously on the server ([c307ea9](https://github.com/sejori/actualised-ai/commit/c307ea9df9c6cc0e4730542bbbafb2f1e61ce2d3))
* **sdk:** async tooling, multi-platform npm packaging, and Release Please CI/CD ([43ea9bf](https://github.com/sejori/actualised-ai/commit/43ea9bfd01226480b28471bba7bdd52a6ff7d302))
* **sdk:** enable async tool execution, multi-platform npm packaging, and Release Please CI/CD ([06f8af4](https://github.com/sejori/actualised-ai/commit/06f8af4626ac3ab1eb3bb2a3b96e64d889cd0ccd))
* setup E2E tests, expose custom tool executor, fix server init ([f2985e6](https://github.com/sejori/actualised-ai/commit/f2985e69758438f9a66d3107ca242f8d0d6b17ea))
* **tests:** add automated batch unit tests and complete Phase 3 verification ([03c706c](https://github.com/sejori/actualised-ai/commit/03c706c0b79af5e77d28513ea118c7bb8e0f51b8))
* **tests:** automated batch unit tests & complete Phase 3 verification ([14e8b17](https://github.com/sejori/actualised-ai/commit/14e8b17469ee6b31e1435001861e8e4d263d3bae))
* unify company state across cloud and browser ([249e767](https://github.com/sejori/actualised-ai/commit/249e7671bb65f42958c42ed44938deffad9d0b09))


### Bug Fixes

* add missing methods to native.d.ts ([b6dc37d](https://github.com/sejori/actualised-ai/commit/b6dc37d09d20534ec11c6e0628bb19331e38618d))
* auth payload property names and dark mode styling ([18cd8a7](https://github.com/sejori/actualised-ai/commit/18cd8a779cf70e42032e396fab49650658d336ca))
* cache system client in telegram webhook to prevent SurrealDB connection exhaustion ([6223b39](https://github.com/sejori/actualised-ai/commit/6223b39acc9db14dad5e2f31ab90404be2d1e21c))
* **core:** update RLS and hydration queries to use dynamic type::string and change UPDATE to UPSERT ([ff09956](https://github.com/sejori/actualised-ai/commit/ff099562a7157ce061f7b42cedf35ed329484e3b))
* dynamic root agent routing in Telegram webhook + typing indicator ([687d26c](https://github.com/sejori/actualised-ai/commit/687d26cf0c0f5c12eaa0ba7d97eb49fe1454a6e7))
* dynamically load correct napi binary based on platform ([039e97d](https://github.com/sejori/actualised-ai/commit/039e97d1c5d0e6dd9f9a2a6e27367ffa1e171d2c))
* ensure schema permissions are applied using root credentials on app boot ([d58465e](https://github.com/sejori/actualised-ai/commit/d58465e0456281c5ede00e27ef093996d813e170))
* load agent config in webhooks and support env var api keys ([2af901d](https://github.com/sejori/actualised-ai/commit/2af901d83ec6b26319c7276a54954c747e517905))
* orchestrator error bubbling and telegram webhook testing ([15cded2](https://github.com/sejori/actualised-ai/commit/15cded2364dd5d5dec9c122a63d22ca5933b0b33))
* orchestrator error recovery and ts loop history ([c746989](https://github.com/sejori/actualised-ai/commit/c7469894893996a2990e9f78e2febcc1ddb46d6b))
* record type casting for surrealdb and queue_message agent id mapping ([c22d5b5](https://github.com/sejori/actualised-ai/commit/c22d5b51a5f9b643099629d863902b1935b7e5fc))
* resolve napi-rs bindings import path ([0151234](https://github.com/sejori/actualised-ai/commit/015123419ac851d9626dd20a86163c6c5a4f92ba))
* restore missing native methods and typescript definitions for github webhooks ([fc805a3](https://github.com/sejori/actualised-ai/commit/fc805a3d96cf6a3a2ca0cfb742b6752f3a02f59e))
* Send orchestrator errors back to Telegram webhook user ([ec5f7ca](https://github.com/sejori/actualised-ai/commit/ec5f7cab681b8a6e701f905259fef25ce854b8b2))
* telegram webhook authentication and background processing lock ([1c1664b](https://github.com/sejori/actualised-ai/commit/1c1664beb1427372ed6cba9271709fa58d79480a))
* **telegram:** implement agentic loop for webhook to resolve tool calls ([85a944f](https://github.com/sejori/actualised-ai/commit/85a944f060e30f6c3d4b674bb2d04ebd93f046d7))
* update tests to include target_company and company_id arguments ([e0d1438](https://github.com/sejori/actualised-ai/commit/e0d1438833186a9d6bd1e6d3b25222e25dbeb00a))
* use IF NOT EXISTS for DEFINE ACCESS to prevent JWT key rotation on cold start ([a26bfa9](https://github.com/sejori/actualised-ai/commit/a26bfa9f119099b8e60e33bcca943bfcd3a1a35c))
