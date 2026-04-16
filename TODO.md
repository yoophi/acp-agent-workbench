# TODO

## Completed

- [x] 현재 Tauri 앱 구조와 참고 Rust ACP POC의 실행 흐름을 확인했다.
- [x] Rust backend를 domain/application/port/adapter 계층으로 분리한 기존 구조를 유지하며 ACP 실행을 연결했다.
- [x] 참고 프로젝트의 agent 기본 목록과 명령을 `AgentCatalog` port 뒤의 static adapter로 제공했다.
- [x] goal textarea, workspace input, agent select, `.txt`/`.md` goal 파일 불러오기, 실행/중지 UI를 연결했다.
- [x] Tauri command로 실행을 시작하고, ACP 스트리밍 메시지를 Tauri event로 frontend에 전달한다.
- [x] ACP 메시지를 `assistant/message`, `tool_call/tool_result`, `usage`, `permission`, `error`, `lifecycle` 중심으로 timeline에 정규화해 표시한다.
- [x] permission 요청에 approve/reject 버튼을 제공하고 Rust backend permission broker로 응답한다.
- [x] permission 응답 처리를 Tauri command에서 application use case로 분리했다.
- [x] 새 실행 시 timeline을 초기화하는 기본 정책을 코드에 명시했다.
- [x] React event listener가 StrictMode에서 중복 등록될 수 있는 경로를 보강했다.
- [x] ACP permission option 선택과 goal 파일 확장자 처리에 단위 테스트를 추가했다.
- [x] `cargo check`, `cargo test`, `npm run build`로 현재 구현을 검증했다.
- [x] permission approve/reject command 실패 시 UI error banner에 오류가 표시되도록 보강했다.
- [x] 실행 종료/취소 시 pending permission waiter를 정리해 오래 열린 UI 세션에서 stale permission 응답이 남지 않게 했다.
- [x] 중지 command 실패 시 UI error banner에 오류가 표시되도록 보강했다.
- [x] `ACP_AGENT_CATALOG_PATH` JSON 파일 provider와 참고 프로젝트 기본 목록 fallback을 갖춘 agent catalog adapter를 추가했다.
- [x] permission broker를 run_id scoped 저장소로 확장해 실행 종료/취소 시 해당 실행의 waiter만 정리한다.
- [x] shadcn/ui CLI 기반 Button 컴포넌트와 `cn` 유틸을 도입해 기존 lightweight shared UI 버튼을 치환했다.
- [x] ACP stdout/read loop 종료 시 pending JSON-RPC request를 오류로 해제해 비정상 종료 hang을 방지했다.

## Remaining

- [ ] ACP session cancel을 단순 task abort 외에 agent별 graceful cancellation/close 프로토콜이 확인되면 확장한다.
- [ ] 실제 agent별 인증 환경에서 장시간 실행과 permission reject 시나리오를 통합 검증한다.
