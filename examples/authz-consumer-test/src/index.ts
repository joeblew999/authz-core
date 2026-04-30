interface Env {
  AUTHZ: Fetcher;
}

const TUPLE = {
  object_type: "document",
  object_id: "binding-probe",
  relation: "owner",
  subject_type: "user",
  subject_id: "alice",
};

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    const url = new URL(req.url);

    if (url.pathname === "/probe/health") {
      const r = await env.AUTHZ.fetch("https://authz/health");
      const body = await r.text();
      return Response.json({ status: r.status, body });
    }

    if (url.pathname === "/probe/tuple") {
      const qs = new URLSearchParams(TUPLE).toString();
      const headers = { "content-type": "application/json" };

      const post = await env.AUTHZ.fetch("https://authz/debug/tuple", {
        method: "POST",
        headers,
        body: JSON.stringify(TUPLE),
      });
      const postBody = await post.json();

      const get = await env.AUTHZ.fetch(`https://authz/debug/tuple?${qs}`);
      const getBody = await get.json();

      const del = await env.AUTHZ.fetch(`https://authz/debug/tuple?${qs}`, {
        method: "DELETE",
      });
      const delBody = await del.json();

      const after = await env.AUTHZ.fetch(`https://authz/debug/tuple?${qs}`);

      return Response.json({
        post: { status: post.status, body: postBody },
        get: { status: get.status, body: getBody },
        delete: { status: del.status, body: delBody },
        get_after_delete: { status: after.status }, // expect 404
      });
    }

    return new Response("Not Found", { status: 404 });
  },
};
