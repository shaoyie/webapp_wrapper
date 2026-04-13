window.addEventListener("DOMContentLoaded", () => {
  const status = document.querySelector<HTMLElement>("[data-status]");
  if (status) {
    status.textContent = "如果看到此页面，说明远程应用未能加载，请检查网络或 URL 配置。";
  }
});
