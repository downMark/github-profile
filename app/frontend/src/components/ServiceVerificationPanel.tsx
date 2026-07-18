import { useCallback, useEffect, useState } from 'react'
import { checkService, serviceChecks } from '../api/services'
import type { ServiceName } from '../types/service'
import './ServiceVerificationPanel.css'

type CheckPhase = 'loading' | 'ready' | 'error'

interface CheckResult {
  service: ServiceName
  label: string
  phase: CheckPhase
  message: string
  environment: string
  revision: string
}

function loadingResults(): CheckResult[] {
  return serviceChecks.map(({ service, label }) => ({
    service,
    label,
    phase: 'loading',
    message: '正在检查服务链路…',
    environment: '—',
    revision: '—',
  }))
}

function statusText(phase: CheckPhase) {
  if (phase === 'ready') return '正常'
  if (phase === 'error') return '异常'
  return '检查中'
}

export default function ServiceVerificationPanel() {
  const [results, setResults] = useState<CheckResult[]>(loadingResults)
  const [checking, setChecking] = useState(true)

  const verify = useCallback(async () => {
    setChecking(true)
    setResults(loadingResults())

    const nextResults = await Promise.all(
      serviceChecks.map(async (definition): Promise<CheckResult> => {
        try {
          const response = await checkService(definition)
          return {
            service: definition.service,
            label: definition.label,
            phase: 'ready',
            message: response.message,
            environment: response.environment,
            revision: response.revision,
          }
        } catch (error) {
          return {
            service: definition.service,
            label: definition.label,
            phase: 'error',
            message: error instanceof Error ? error.message : '服务检查失败',
            environment: '—',
            revision: '—',
          }
        }
      }),
    )

    setResults(nextResults)
    setChecking(false)
  }, [])

  useEffect(() => {
    void verify()
  }, [verify])

  return (
    <section className="service-verification" aria-labelledby="service-verification-title">
      <div className="service-verification__header">
        <div>
          <span className="service-verification__eyebrow">DEPLOYMENT CHECK</span>
          <h2 id="service-verification-title">服务联调验证</h2>
          <p>
            当前站点：<code>{window.location.hostname}</code>
          </p>
        </div>
        <button type="button" disabled={checking} onClick={() => void verify()}>
          {checking ? '检查中…' : '重新检查'}
        </button>
      </div>

      <ul className="service-verification__grid">
        {results.map((result) => (
          <li key={result.service} className={`service-verification__item service-verification__item--${result.phase}`}>
            <div className="service-verification__item-header">
              <strong>{result.label}</strong>
              <span>{statusText(result.phase)}</span>
            </div>
            <p>{result.message}</p>
            <dl className="service-verification__metadata">
              <div><dt>环境</dt><dd>{result.environment}</dd></div>
              <div><dt>版本</dt><dd title={result.revision}>{result.revision.slice(0, 12)}</dd></div>
            </dl>
          </li>
        ))}
      </ul>
    </section>
  )
}
