export function findUserEmail(db: { query: (q: string) => unknown }) {
  return db.query(`SELECT users.email, users.email_address FROM users WHERE users.id = $1`);
}
