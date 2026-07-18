import { useState, type FormEvent } from 'react'
import { Link, Navigate, useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from '../auth/AuthContext'
import './AuthPage.css'

export default function AuthPage({ mode }: { mode: 'login' | 'register' }) {
  const { account, login, register } = useAuth()
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [submitting, setSubmitting] = useState(false)
  const navigate = useNavigate()
  const location = useLocation()
  if (account) return <Navigate to="/" replace />
  const isRegister = mode === 'register'

  async function submit(event: FormEvent) {
    event.preventDefault(); setSubmitting(true); setError('')
    try {
      if (isRegister) await register(username, password); else await login(username, password)
      const from = (location.state as { from?: string } | null)?.from
      navigate(from || '/', { replace: true })
    } catch (err) { setError(err instanceof Error ? err.message : '操作失败，请重试') }
    finally { setSubmitting(false) }
  }

  return <main className="auth-page">
    <form className="auth-card" onSubmit={submit}>
      <div className="auth-card__brand">GitHub Profile Manager</div>
      <h1>{isRegister ? '创建系统账号' : '登录'}</h1>
      <p>登录后导入 GitHub 账号，并为每个账号管理 Todo。</p>
      <label>用户名<input value={username} onChange={e => setUsername(e.target.value)} autoComplete="username" required minLength={3} maxLength={32} /></label>
      <label>密码（7–128 个字符）<input type="password" value={password} onChange={e => setPassword(e.target.value)} autoComplete={isRegister ? 'new-password' : 'current-password'} required minLength={7} maxLength={128} /></label>
      {error && <div className="auth-card__error" role="alert">{error}</div>}
      <button disabled={submitting}>{submitting ? '请稍候…' : isRegister ? '注册并登录' : '登录'}</button>
      <span>{isRegister ? '已有账号？' : '还没有账号？'} <Link to={isRegister ? '/login' : '/register'}>{isRegister ? '去登录' : '创建账号'}</Link></span>
    </form>
  </main>
}
