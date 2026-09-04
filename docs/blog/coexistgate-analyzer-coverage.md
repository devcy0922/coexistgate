---
title: "분석기가 모르는 변경을 PASS로 만들지 않기"
date: 2026-09-05
description: "CoexistGate의 analyzer coverage를 보강하고, 불완전한 증거가 안전한 릴리스로 오인되지 않게 만든 기록."
tags:
  - Backend
  - Developer Tools
  - Reliability
  - Rust
---

# 분석기가 모르는 변경을 PASS로 만들지 않기

Release gate의 가장 위험한 버그는 잘못된 FAIL이 아닐 수 있다. 분석기가 파일을 읽었지만 이해하지 못했고, 그 결과를 “변경 없음”으로 처리해 PASS를 내는 경우다.

## 빈 결과는 안전의 증거가 아니다

CoexistGate의 핵심 흐름은 파일 트리에서 사실을 추출하고, 이전 애플리케이션·후보 스키마·배포·정책 사이의 관계를 검사하는 것이다.

```text
Base + Candidate FileTree
            │
            ▼
      Artifact Discovery
            │
            ▼
         Analyzers
          ┌───┴───┐
       Facts   Issues
          │       │
          ▼       ▼
     ReleaseModel  ───────┐
          │               │
          ▼               ▼
    Cross-artifact Rules  Coverage Finding
          └───────┬───────┘
                  ▼
            Deterministic Gate
                  │
                  ▼
               Report + Evidence
```

이제 분석기는 사실만 반환하지 않는다. 파서가 지원하지 않는 문법, 손상된 YAML, 잘못된 `.env` assignment처럼 안전성을 증명할 수 없는 상황도 함께 반환한다.

## fail-closed 경계

분석 불확실성은 `ANALYZER-COVERAGE-001` finding으로 변환된다. 이 finding은 파일 경로, 라인, 분석기, 실패 사유를 evidence로 포함하며 기본 high 심각도로 게이트를 차단한다.

```text
ALTER TABLE users ENABLE ROW LEVEL SECURITY;

→ ANALYZER-COVERAGE-001 / HIGH
  Release evidence is incomplete
  migration analyzer could not safely interpret
  unsupported or ambiguous PostgreSQL DDL statement

→ Gate: FAIL
```

정책은 알려진 위험의 허용 수준을 조정할 수 있지만, 분석되지 않은 입력을 안전하다고 간주하도록 만들 수는 없다.

## 정상적인 PASS는 유지한다

모든 미인식 SQL을 무조건 막는 것도 올바른 해법은 아니다. baseline의 `CREATE TABLE` 선언과 expand 단계의 nullable `ADD COLUMN`처럼 제품이 이해하고 안전한 범위로 정의한 입력은 기존 PASS 동작을 유지한다.

목표는 다음의 구분이다.

- 알고 있고 안전함 → 정상 평가
- 알고 있고 위험함 → cross-artifact finding
- 변경이 있지만 해석하지 못함 → coverage finding, FAIL

## 검증

기존 unsafe DB, safe expand, availability regression, config rollback fixture를 유지하면서 unsupported DDL 회귀 테스트를 추가했다.

```text
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

CoexistGate의 질문은 “이 파일이 lint를 통과했는가?”가 아니다. “이 릴리스가 안전하다는 결론을 낼 충분한 증거가 있는가?”다.

[CoexistGate GitHub 저장소](https://github.com/devcy0922/coexistgate)
