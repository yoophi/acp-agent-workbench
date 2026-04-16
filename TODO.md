# TODO

## Completed

- [x] 현재 Tauri 앱 구조와 참고 Rust ACP POC의 실행 흐름을 확인했다.
- [x] Rust backend를 domain/application/port/adapter 계층으로 분리한 기존 구조를 유지하며 ACP 실행을 연결했다.
- [x] 참고 프로젝트의 agent 기본 목록과 명령을 `AgentCatalog` port 뒤의 static adapter로 제공했다.
- [x] goal textarea, workspace input, agent select, `.txt`/`.md` goal 파일 불러오기, 실행/중지 UI를 연결했다.
- [x] Tauri command로 실행을 시작하고, ACP 스트리밍 메시지를 Tauri event로 frontend에 전달한다.
- [x] ACP 메시지를 `assistant/message`, `tool_call/tool_result`, `usage`, `permission`, `error`, `lifecycle` 중심으로 timeline에 정규화해 표시한다.
- [x] permission 요청에 approve/reject 버튼을 제공하고 Rust backend permission broker로 응답한다.
- [x] 새 실행 시 timeline을 초기화하는 기본 정책을 코드에 명시했다.
- [x] React event listener가 StrictMode에서 중복 등록될 수 있는 경로를 보강했다.

## Remaining

- [ ] `StaticAgentCatalog`를 설정 파일 또는 provider 기반 adapter로 교체한다.
- [ ] ACP session cancel을 단순 task abort 외에 agent별 graceful cancellation/close 프로토콜이 확인되면 확장한다.
- [ ] shadcn/ui CLI 기반 컴포넌트 세트를 도입해 현재 lightweight shared UI를 공식 shadcn 컴포넌트로 치환한다.
- [ ] 실제 agent별 인증 환경에서 장시간 실행과 permission reject 시나리오를 통합 검증한다.
