import { useCallback, useEffect, useState } from 'react'
import { listUsers } from '../api/users'
import type { GithubUser } from '../types/user'
import UserCard from '../components/UserCard'
import TokenInput from '../components/TokenInput'
import './UserListPage.css'

const PAGE_LIMIT = 20

type LoadState = 'loading' | 'error' | 'ready'

/** 用户列表页：卡片网格 + 分页，点击卡片跳转详情（design.md 模块 2 / T-010） */
export default function UserListPage() {
  const [users, setUsers] = useState<GithubUser[]>([])
  const [page, setPage] = useState(1)
  const [state, setState] = useState<LoadState>('loading')
  const [errorMessage, setErrorMessage] = useState('')
  const [maybeHasNext, setMaybeHasNext] = useState(false)

  const load = useCallback(async (targetPage: number) => {
    setState('loading')
    setErrorMessage('')
    try {
      const result = await listUsers(targetPage, PAGE_LIMIT)
      setUsers(result.items)
      setMaybeHasNext(targetPage * PAGE_LIMIT < result.total)
      setState('ready')
    } catch (err) {
      setErrorMessage(err instanceof Error ? err.message : '加载用户列表失败')
      setState('error')
    }
  }, [])

  useEffect(() => {
    void load(page)
  }, [load, page])

  return (
    <main className="user-list-page">
      <header className="user-list-page__header">
        <h1>用户列表</h1>
        <p>已入库的 GitHub 用户，点击卡片查看详情</p>
      </header>

      <TokenInput
        onSuccess={() => {
          if (page === 1) void load(1)
          else setPage(1)
        }}
      />

      {state === 'loading' && (
        <p className="user-list-page__status" role="status">
          加载中…
        </p>
      )}

      {state === 'error' && (
        <div className="user-list-page__status user-list-page__status--error" role="alert">
          <p>{errorMessage}</p>
          <button type="button" onClick={() => void load(page)}>
            重试
          </button>
        </div>
      )}

      {state === 'ready' && users.length === 0 && (
        <p className="user-list-page__status">
          {page > 1 ? '这一页没有数据了' : '暂无用户，先通过 Token 导入一个吧'}
        </p>
      )}

      {state === 'ready' && users.length > 0 && (
        <ul className="user-list-page__grid">
          {users.map((user) => (
            <li key={user.id}>
              <UserCard user={user} />
            </li>
          ))}
        </ul>
      )}

      <nav className="user-list-page__pagination" aria-label="分页">
        <button
          type="button"
          disabled={page <= 1 || state === 'loading'}
          onClick={() => setPage((p) => Math.max(1, p - 1))}
        >
          上一页
        </button>
        <span>第 {page} 页</span>
        <button
          type="button"
          disabled={!maybeHasNext || state !== 'ready'}
          onClick={() => setPage((p) => p + 1)}
        >
          下一页
        </button>
      </nav>
    </main>
  )
}
