/** 用户对象，字段与后端接口契约一致（design.md「接口契约」），不含 encrypted_token */
export interface GithubUser {
  id: string
  github_id: number
  login: string
  name: string | null
  bio: string | null
  avatar_url: string | null
  html_url: string | null
  public_repos: number
  followers: number
  following: number
  company: string | null
  blog: string | null
  location: string | null
  /** GitHub 账号注册时间 */
  github_created_at: string
  /** 首次入库时间 */
  created_at: string
  updated_at: string
}
