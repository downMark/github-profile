import { Link } from 'react-router-dom'
import type { GithubUser } from '../types/user'
import './UserCard.css'

interface UserCardProps {
  user: GithubUser
}

function formatStoredAt(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return iso
  return date.toLocaleString('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
  })
}

/** 用户列表卡片：头像 / 用户名 / bio / 入库时间，点击跳转详情（design.md 模块 2） */
export default function UserCard({ user }: UserCardProps) {
  return (
    <Link className="user-card" to={`/users/${user.id}`}>
      {user.avatar_url ? (
        <img
          className="user-card__avatar"
          src={user.avatar_url}
          alt={`${user.login} 的头像`}
          width="64"
          height="64"
          loading="lazy"
        />
      ) : (
        <div
          className="user-card__avatar user-card__avatar--placeholder"
          aria-hidden="true"
        >
          {user.login.charAt(0).toUpperCase()}
        </div>
      )}
      <div className="user-card__body">
        <h3 className="user-card__name">{user.name ?? user.login}</h3>
        <p className="user-card__login">@{user.login}</p>
        <p className="user-card__bio">{user.bio ?? '这个人很神秘，什么都没写。'}</p>
        <time className="user-card__time" dateTime={user.created_at}>
          入库于 {formatStoredAt(user.created_at)}
        </time>
      </div>
    </Link>
  )
}
