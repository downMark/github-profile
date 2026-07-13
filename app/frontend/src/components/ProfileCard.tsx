import type { GithubUser } from '../types/user'
import './ProfileCard.css'

interface ProfileCardProps {
  user: GithubUser
}

function formatDate(iso: string): string {
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return iso
  return date.toLocaleDateString('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })
}

function normalizeBlogUrl(blog: string): string {
  return /^https?:\/\//i.test(blog) ? blog : `https://${blog}`
}

/** 格式化个人介绍卡片：全部来自已入库数据，无额外 API 调用（design.md 模块 4） */
export default function ProfileCard({ user }: ProfileCardProps) {
  return (
    <article className="profile-card">
      <header className="profile-card__header">
        {user.avatar_url ? (
          <img
            className="profile-card__avatar"
            src={user.avatar_url}
            alt={`${user.login} 的头像`}
            width="96"
            height="96"
          />
        ) : (
          <div className="profile-card__avatar profile-card__avatar--placeholder" aria-hidden="true">
            {user.login.charAt(0).toUpperCase()}
          </div>
        )}
        <div className="profile-card__identity">
          <h2 className="profile-card__name">{user.name ?? user.login}</h2>
          {user.html_url && (
            <a
              className="profile-card__login"
              href={user.html_url}
              target="_blank"
              rel="noreferrer"
            >
              @{user.login}
            </a>
          )}
        </div>
      </header>

      {user.bio && <p className="profile-card__bio">{user.bio}</p>}

      <dl className="profile-card__stats">
        <div className="profile-card__stat">
          <dt>公开仓库</dt>
          <dd>{user.public_repos}</dd>
        </div>
        <div className="profile-card__stat">
          <dt>Followers</dt>
          <dd>{user.followers}</dd>
        </div>
        <div className="profile-card__stat">
          <dt>Following</dt>
          <dd>{user.following}</dd>
        </div>
      </dl>

      <ul className="profile-card__meta">
        {user.company && <li>🏢 {user.company}</li>}
        {user.location && <li>📍 {user.location}</li>}
        {user.blog && (
          <li>
            🔗{' '}
            <a href={normalizeBlogUrl(user.blog)} target="_blank" rel="noreferrer">
              {user.blog}
            </a>
          </li>
        )}
        <li>📅 账号创建于 {formatDate(user.github_created_at)}</li>
      </ul>
    </article>
  )
}
