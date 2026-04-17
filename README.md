# ACP Agent Workbench

Tauri 기반의 Agent Client Protocol(ACP) 에이전트 실행 워크벤치입니다. React UI에서 목표 프롬프트를 입력하거나 파일로 불러오고, 선택한 ACP 에이전트를 로컬 작업 디렉터리에서 실행한 뒤 ACP 메시지 스트림과 권한 요청을 확인할 수 있습니다.

## 주요 기능

- ACP 에이전트 선택 및 실행
- 목표 프롬프트 직접 입력 또는 `.txt`, `.md` 파일 로드
- 실행 작업 디렉터리 지정
- 에이전트 실행 명령 오버라이드
- stdout 기반 ACP 메시지 스트림 표시
- stderr 진단 메시지 표시
- 권한 요청 승인/거절
- 권한 요청 자동 허용 옵션
- 실행 중인 에이전트 프로세스 중지
- 메시지 그룹별 필터링

## 기술 스택

- Tauri 2
- Rust 2024
- React 19
- TypeScript
- Vite
- Tailwind CSS
- TanStack Query
- Zustand

## 사전 준비

- Node.js 및 npm
- Rust toolchain
- Tauri 2 개발 환경

ACP 에이전트는 기본적으로 `npx` 명령으로 실행되므로 네트워크 접근과 해당 패키지 실행 권한이 필요합니다.

## 설치

```sh
npm install
```

## 개발 실행

```sh
npm run tauri:dev
```

`npm run tauri:dev`는 `1420`부터 사용 가능한 포트를 찾아 Tauri의 `devUrl`과 Vite 개발 서버 포트를 함께 설정합니다. 따라서 이미 개발용 앱이 떠 있어도 다른 포트로 추가 인스턴스를 실행할 수 있습니다.

여러 개발용 앱을 동시에 실행할 때는 `npm run tauri dev` 대신 `npm run tauri:dev`를 사용해야 합니다. 기존 `npm run tauri dev`는 `src-tauri/tauri.conf.json`의 고정 `1420` 포트를 그대로 사용합니다.

특정 시작 포트를 지정하려면 다음처럼 실행합니다.

```sh
TAURI_DEV_PORT=1430 npm run tauri:dev
```

실제 앱을 띄우지 않고 선택될 Tauri dev 설정만 확인하려면 다음 명령을 사용할 수 있습니다.

```sh
npm run tauri:dev -- --print-config
```

프론트엔드만 실행하려면 다음 명령을 사용할 수 있습니다.

```sh
npm run dev
```

## 개발용 컴포넌트 선택 (react-grab)

개발 모드(`npm run dev`, `npm run tauri:dev`)에서는 [`react-grab`](https://react-grab.com)이 자동으로 활성화됩니다. UI 요소에 마우스를 올린 뒤 macOS는 `⌘C`, Windows/Linux는 `Ctrl+C`를 누르면 해당 요소의 파일 경로, React 컴포넌트 이름, HTML 소스가 클립보드에 복사되어 코딩 에이전트에게 바로 붙여 넣을 수 있습니다.

`src/main.tsx`에서 `import.meta.env.DEV`가 `true`일 때만 동적으로 import 하므로 프로덕션 번들에는 포함되지 않습니다.

## 빌드

```sh
npm run build
```

Tauri 앱 빌드는 다음 명령을 사용합니다.

```sh
npm run tauri build
```

현재 `src-tauri/tauri.conf.json`의 `bundle.active` 값은 `false`입니다. 배포 번들을 만들려면 Tauri 번들 설정을 먼저 조정해야 합니다.

## 기본 에이전트

별도 설정이 없으면 다음 ACP 에이전트 목록을 사용합니다.

| ID | 이름 | 실행 명령 |
| --- | --- | --- |
| `claude-code` | Claude Code | `npx -y @agentclientprotocol/claude-agent-acp` |
| `codex` | Codex | `npx -y @zed-industries/codex-acp` |
| `opencode` | OpenCode | `npx -y opencode-ai acp` |
| `pi` | Pi | `npx -y pi-acp` |

## 커스텀 에이전트 카탈로그

`ACP_AGENT_CATALOG_PATH` 환경 변수에 JSON 파일 경로를 지정하면 기본 에이전트 목록 대신 해당 파일을 읽습니다.

```sh
ACP_AGENT_CATALOG_PATH=/path/to/agents.json npm run tauri:dev
```

파일 형식은 다음과 같습니다.

```json
[
  {
    "id": "local-agent",
    "label": "Local Agent",
    "command": "local-agent acp"
  }
]
```

파일을 읽을 수 없거나 JSON 형식이 올바르지 않거나 목록이 비어 있으면 기본 에이전트 목록으로 돌아갑니다.

## 실행 옵션

- `Agent`: 실행할 ACP 에이전트입니다.
- `Workspace`: 에이전트 프로세스가 실행될 작업 디렉터리입니다. 비워 두면 `~/tmp/acp-tauri-agent-workspace`를 사용합니다.
- `Command override`: 선택한 에이전트의 기본 실행 명령 대신 사용할 명령입니다.
- `Stdio buffer`: ACP stdout 읽기 버퍼 한도입니다. UI에서는 1MB부터 512MB까지 입력할 수 있습니다.
- `Auto-select allow permission`: 에이전트가 권한 요청을 보낼 때 허용 옵션을 자동 선택합니다.

## 프로젝트 구조

```text
src/
  app/                    React 앱 진입 및 전역 스타일
  entities/               프론트엔드 도메인 모델과 메시지 포맷터
  features/               목표 입력과 에이전트 실행 상태 관리
  pages/agent-workbench/  워크벤치 페이지
  shared/                 Tauri API 래퍼와 공용 UI
  widgets/                이벤트 스트림, 실행 패널

src-tauri/
  src/adapters/           ACP, Tauri, 파일 시스템, 에이전트 카탈로그 어댑터
  src/application/        유스케이스
  src/domain/             도메인 모델과 이벤트
  src/ports/              포트 인터페이스
```

## 참고

- 앱 이름은 `ACP Agent Workbench`입니다.
- 패키지 이름과 Tauri identifier는 `acp-agent-workbench` 기준으로 설정되어 있습니다.
- 현재 저장소에는 별도 테스트 스크립트가 정의되어 있지 않습니다.
