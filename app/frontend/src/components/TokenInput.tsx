import { useState, type FormEvent } from 'react'
import { importUser } from '../api/users'
import { ApiError } from '../api/client'
import type { GithubUser } from '../types/user'
import './TokenInput.css'

export interface TokenInputProps {
  /** 导入成功回调（如刷新用户列表、跳转详情） */
  onSuccess?: (user: GithubUser) => void
}

/**
 * GitHub Token 输入组件（F-001 / AC-002）
 * - type="password" 掩码显示，前端不落地明文 token
 * - 提交调用 POST /api/users，展示 loading 状态
 * - 失败时展示后端返回的错误信息（无效 token → 明确提示）
 */
function TokenInput({ onSuccess }: TokenInputProps) {
  const [token, setToken] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const canSubmit = token.trim().length > 0 && !loading

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!canSubmit) return

    setLoading(true)
    setError(null)
    try {
      const user = await importUser(token.trim())
      setToken('')
      onSuccess?.(user)
    } catch (err) {
      setError(
        err instanceof ApiError ? err.message : '导入失败，请稍后重试',
      )
    } finally {
      setLoading(false)
    }
  }

  return (
    <form className="token-input" onSubmit={handleSubmit} noValidate>
      <label className="token-input-label" htmlFor="github-token">
        GitHub Token
      </label>
      <div className="token-input-row">
        <input
          id="github-token"
          className="token-input-field"
          type="password"
          value={token}
          onChange={(event) => {
            setToken(event.target.value)
            if (error) setError(null)
          }}
          placeholder="输入 GitHub Personal Access Token"
          autoComplete="off"
          spellCheck={false}
          disabled={loading}
          aria-invalid={error !== null}
          aria-describedby={error ? 'github-token-error' : undefined}
        />
        <button
          className="token-input-submit"
          type="submit"
          disabled={!canSubmit}
        >
          {loading ? '导入中…' : '导入'}
        </button>
      </div>
      {error && (
        <p id="github-token-error" className="token-input-error" role="alert">
          {error}
        </p>
      )}
    </form>
  )
}

export default TokenInput
