
(()=>{
const root=document.getElementById('anima-console'),main=root.querySelector('main'),nav=root.querySelector('nav'),overlay=root.querySelector('.overlay');
const names=['Overview','Network','Anima','Storage','Display','Recovery'];
const preference=matchMedia('(prefers-reduced-motion: reduce)');
let motionChoice='system',dialogVersion=0,returnFocus=null,closing=false;
const activeMotion=new Set();
const reduced=()=>motionChoice==='reduced'||(motionChoice==='system'&&preference.matches);
const role={press:100,fast:150,base:200};
function animate(element,frames,duration=role.base){
 if(reduced())return Promise.resolve();
 const animation=element.animate(frames,{duration,easing:'cubic-bezier(.2,0,0,1)'});
 activeMotion.add(animation);
 return animation.finished.catch(()=>{}).finally(()=>activeMotion.delete(animation));
}
function syncMotion(){
 root.dataset.reduced=String(reduced());
 if(reduced())for(const animation of activeMotion)animation.finish();
}
preference.addEventListener('change',syncMotion);
syncMotion();
function backgroundInert(value){
 for(const element of root.querySelectorAll('header,nav,main,footer'))element.inert=value;
}
function showPage(html){
 main.querySelectorAll('.page-outgoing').forEach(element=>element.remove());
 const old=main.querySelector('.page-content');
 if(old&&!reduced()){
  old.className='page-outgoing';old.inert=true;old.setAttribute('aria-hidden','true');
  old.querySelectorAll('[id]').forEach(element=>element.removeAttribute('id'));
  animate(old,[{opacity:1},{opacity:0}],role.fast).then(()=>old.remove());
 }else old?.remove();
 const content=document.createElement('div');content.className='page-content';content.innerHTML=html;
 main.append(content);
 animate(content,[{opacity:0},{opacity:1}]);
}

let page='Overview',timer=null;
const icons={"Overview": "data:image/svg+xml;base64,PCEtLSBJQk0gQ2FyYm9uLCBBcGFjaGUtMi4wOyBleGlzdGluZyBBbmltYSBpY29uIGFzc2V0LiAtLT48c3ZnIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDMyIDMyIiBmaWxsPSIjQTRBOEIwIj48cGF0aCBkPSJNMjQgMjFoMnY1aC0yem0tNC01aDJ2MTBoLTJ6bS05IDEwYTUuMDA2IDUuMDA2IDAgMCAxLTUtNWgyYTMgMyAwIDEgMCAzLTN2LTJhNSA1IDAgMCAxIDAgMTAiIC8+PHBhdGggZD0iTTI4IDJINGEyIDIgMCAwIDAtMiAydjI0YTIgMiAwIDAgMCAyIDJoMjRhMi4wMDMgMi4wMDMgMCAwIDAgMi0yVjRhMiAyIDAgMCAwLTItMm0wIDlIMTRWNGgxNFpNMTIgNHY3SDRWNFpNNCAyOFYxM2gyNGwuMDAyIDE1WiIgLz48L3N2Zz4=", "Network": "data:image/svg+xml;base64,PCEtLSBJQk0gQ2FyYm9uLCBBcGFjaGUtMi4wOyBleGlzdGluZyBBbmltYSBpY29uIGFzc2V0LiAtLT48c3ZnIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDMyIDMyIiBmaWxsPSIjQTRBOEIwIj48cGF0aCBkPSJNMzAgMzBoLTh2LThoOFptLTYtMmg0di00aC00Wm0tNC0xSDhhNiA2IDAgMCAxIDAtMTJoMnYySDhhNCA0IDAgMCAwIDAgOGgxMloiIC8+PHBhdGggZD0iTTIwIDIwaC04di04aDhabS02LTJoNHYtNGgtNFoiIC8+PHBhdGggZD0iTTI0IDE3aC0ydi0yaDJhNCA0IDAgMCAwIDAtOEgxMlY1aDEyYTYgNiAwIDAgMSAwIDEybS0xNC03SDJWMmg4Wk00IDhoNFY0SDRaIiAvPjwvc3ZnPg==", "Anima": "data:image/svg+xml;base64,PCEtLSBJQk0gQ2FyYm9uLCBBcGFjaGUtMi4wOyBleGlzdGluZyBBbmltYSBpY29uIGFzc2V0LiAtLT48c3ZnIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDMyIDMyIiBmaWxsPSIjQTRBOEIwIj48cGF0aCBkPSJNMTUgMTJoMnYxOGgtMnptLTMuNjcgNi4yMmE3IDcgMCAwIDEgMC0xMC40NGwxLjM0IDEuNDlhNSA1IDAgMCAwIDAgNy40NlptOS4zNCAwbC0xLjM0LTEuNDlhNSA1IDAgMCAwIDAtNy40NmwxLjM0LTEuNDlhNyA3IDAgMCAxIDAgMTAuNDQiIC8+PHBhdGggZD0iTTguNCAyMS44YTExIDExIDAgMCAxIDAtMTcuNmwxLjIgMS42YTkgOSAwIDAgMCAwIDE0LjRabTE1LjIgMGwtMS4yLTEuNmE5IDkgMCAwIDAgMC0xNC40bDEuMi0xLjZhMTEgMTEgMCAwIDEgMCAxNy42IiAvPjwvc3ZnPg==", "Storage": "data:image/svg+xml;base64,PCEtLSBJQk0gQ2FyYm9uLCBBcGFjaGUtMi4wOyBleGlzdGluZyBBbmltYSBpY29uIGFzc2V0LiAtLT48c3ZnIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDMyIDMyIiBmaWxsPSIjQTRBOEIwIj48cGF0aCBkPSJNMjQgM0g4YTIgMiAwIDAgMC0yIDJ2MjJhMiAyIDAgMCAwIDIgMmgxNmEyIDIgMCAwIDAgMi0yVjVhMiAyIDAgMCAwLTItMm0wIDJ2Nkg4VjVaTTggMTl2LTZoMTZ2NlptMCA4di02aDE2djZaIiAvPjxjaXJjbGUgY3g9IjExIiBjeT0iOCIgcj0iMSIgLz48Y2lyY2xlIGN4PSIxMSIgY3k9IjE2IiByPSIxIiAvPjxjaXJjbGUgY3g9IjExIiBjeT0iMjQiIHI9IjEiIC8+PC9zdmc+", "Display": "data:image/svg+xml;base64,PCEtLSBJQk0gQ2FyYm9uLCBBcGFjaGUtMi4wOyBleGlzdGluZyBBbmltYSBpY29uIGFzc2V0LiAtLT48c3ZnIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDMyIDMyIiBmaWxsPSIjQTRBOEIwIj48cGF0aCBkPSJNMjggNEg0YTIgMiAwIDAgMC0yIDJ2MTZhMiAyIDAgMCAwIDIgMmg4djRIOHYyaDE2di0yaC00di00aDhhMiAyIDAgMCAwIDItMlY2YTIgMiAwIDAgMC0yLTJNMTggMjhoLTR2LTRoNFptMTAtNkg0VjZoMjRaIiAvPjwvc3ZnPg==", "Recovery": "data:image/svg+xml;base64,PCEtLSBJQk0gQ2FyYm9uLCBBcGFjaGUtMi4wOyBleGlzdGluZyBBbmltYSBpY29uIGFzc2V0LiAtLT48c3ZnIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDMyIDMyIiBmaWxsPSIjQTRBOEIwIj48cGF0aCBkPSJNMjcgMTYuNzZ2LTEuNTNsMS45Mi0xLjY4QTIgMiAwIDAgMCAyOS4zIDExbC0yLjM2LTRhMiAyIDAgMCAwLTEuNzMtMWEyIDIgMCAwIDAtLjY0LjFsLTIuNDMuODJhMTEgMTEgMCAwIDAtMS4zMS0uNzVsLS41MS0yLjUyYTIgMiAwIDAgMC0yLTEuNjFoLTQuNjhhMiAyIDAgMCAwLTIgMS42MWwtLjUxIDIuNTJhMTEuNSAxMS41IDAgMCAwLTEuMzIuNzVsLTIuMzgtLjg2QTIgMiAwIDAgMCA2Ljc5IDZhMiAyIDAgMCAwLTEuNzMgMUwyLjcgMTFhMiAyIDAgMCAwIC40MSAyLjUxTDUgMTUuMjR2MS41M2wtMS44OSAxLjY4QTIgMiAwIDAgMCAyLjcgMjFsMi4zNiA0YTIgMiAwIDAgMCAxLjczIDFhMiAyIDAgMCAwIC42NC0uMWwyLjQzLS44MmExMSAxMSAwIDAgMCAxLjMxLjc1bC41MSAyLjUyYTIgMiAwIDAgMCAyIDEuNjFoNC43MmEyIDIgMCAwIDAgMi0xLjYxbC41MS0yLjUyYTExLjUgMTEuNSAwIDAgMCAxLjMyLS43NWwyLjQyLjgyYTIgMiAwIDAgMCAuNjQuMWEyIDIgMCAwIDAgMS43My0xbDIuMjgtNGEyIDIgMCAwIDAtLjQxLTIuNTFaTTI1LjIxIDI0bC0zLjQzLTEuMTZhOC45IDguOSAwIDAgMS0yLjcxIDEuNTdMMTguMzYgMjhoLTQuNzJsLS43MS0zLjU1YTkuNCA5LjQgMCAwIDEtMi43LTEuNTdMNi43OSAyNGwtMi4zNi00bDIuNzItMi40YTguOSA4LjkgMCAwIDEgMC0zLjEzTDQuNDMgMTJsMi4zNi00bDMuNDMgMS4xNmE4LjkgOC45IDAgMCAxIDIuNzEtMS41N0wxMy42NCA0aDQuNzJsLjcxIDMuNTVhOS40IDkuNCAwIDAgMSAyLjcgMS41N0wyNS4yMSA4bDIuMzYgNGwtMi43MiAyLjRhOC45IDguOSAwIDAgMSAwIDMuMTNMMjcuNTcgMjBaIiAvPjxwYXRoIGQ9Ik0xNiAyMmE2IDYgMCAxIDEgNi02YTUuOTQgNS45NCAwIDAgMS02IDZtMC0xMGEzLjkxIDMuOTEgMCAwIDAtNCA0YTMuOTEgMy45MSAwIDAgMCA0IDRhMy45MSAzLjkxIDAgMCAwIDQtNGEzLjkxIDMuOTEgMCAwIDAtNC00IiAvPjwvc3ZnPg=="};
Object.assign(icons,{"locked": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTI0IDE0aC0yVjhhNiA2IDAgMCAwLTEyIDB2Nkg4YTIgMiAwIDAgMC0yIDJ2MTJhMiAyIDAgMCAwIDIgMmgxNmEyIDIgMCAwIDAgMi0yVjE2YTIgMiAwIDAgMC0yLTJNMTIgOGE0IDQgMCAwIDEgOCAwdjZoLThabTEyIDIwSDhWMTZoMTZaIiAvPjwvc3ZnPg==", "password": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTIxIDJhOC45OTggOC45OTggMCAwIDAtOC42MTIgMTEuNjEyTDIgMjR2Nmg2bDEwLjM4OC0xMC4zODhBOSA5IDAgMSAwIDIxIDJtMCAxNmE3IDcgMCAwIDEtMi4wMzItLjMwMmwtMS4xNDctLjM0OGwtLjg0Ny44NDdsLTMuMTgxIDMuMTgxTDEyLjQxNCAyMEwxMSAyMS40MTRsMS4zNzkgMS4zNzlsLTEuNTg2IDEuNTg2TDkuNDE0IDIzTDggMjQuNDE0bDEuMzc5IDEuMzc5TDcuMTcyIDI4SDR2LTMuMTcybDkuODAyLTkuODAybC44NDgtLjg0N2wtLjM0OC0xLjE0N0E3IDcgMCAxIDEgMjEgMTgiIC8+PGNpcmNsZSBjeD0iMjIiIGN5PSIxMCIgcj0iMiIgLz48L3N2Zz4=", "power": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTIyLjUgNS43NGwtMSAxLjczYTExIDExIDAgMSAxLTExIDBsLTEtMS43M2ExMyAxMyAwIDEgMCAxMyAwIiAvPjxwYXRoIGQ9Ik0xNSAyaDJ2MTRoLTJ6IiAvPjwvc3ZnPg==", "renew": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTEyIDEwSDYuNzhBMTEgMTEgMCAwIDEgMjcgMTZoMkExMyAxMyAwIDAgMCA2IDcuNjhWNEg0djhoOFptOCAxMmg1LjIyQTExIDExIDAgMCAxIDUgMTZIM2ExMyAxMyAwIDAgMCAyMyA4LjMyVjI4aDJ2LThoLThaIiAvPjwvc3ZnPg==", "user-avatar": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE2IDhhNSA1IDAgMSAwIDUgNWE1IDUgMCAwIDAtNS01bTAgOGEzIDMgMCAxIDEgMy0zYTMuMDAzIDMuMDAzIDAgMCAxLTMgMyIgLz48cGF0aCBkPSJNMTYgMmExNCAxNCAwIDEgMCAxNCAxNEExNC4wMTYgMTQuMDE2IDAgMCAwIDE2IDJtLTYgMjQuMzc3VjI1YTMuMDAzIDMuMDAzIDAgMCAxIDMtM2g2YTMuMDAzIDMuMDAzIDAgMCAxIDMgM3YxLjM3N2ExMS45IDExLjkgMCAwIDEtMTIgMG0xMy45OTMtMS40NTFBNSA1IDAgMCAwIDE5IDIwaC02YTUgNSAwIDAgMC00Ljk5MiA0LjkyNmExMiAxMiAwIDEgMSAxNS45ODUgMCIgLz48L3N2Zz4=", "add": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE3IDE1VjhoLTJ2N0g4djJoN3Y3aDJ2LTdoN3YtMnoiIC8+PC9zdmc+", "arrow-right": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTE4IDZsLTEuNDMgMS4zOTNMMjQuMTUgMTVINHYyaDIwLjE1bC03LjU4IDcuNTczTDE4IDI2bDEwLTEweiIgLz48L3N2Zz4=", "arrow-left": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTE0IDI2bDEuNDEtMS40MUw3LjgzIDE3SDI4di0ySDcuODNsNy41OC03LjU5TDE0IDZMNCAxNnoiIC8+PC9zdmc+", "checkmark": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTEzIDI0bC05LTlsMS40MTQtMS40MTRMMTMgMjEuMTcxTDI2LjU4NiA3LjU4NkwyOCA5eiIgLz48L3N2Zz4=", "close": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE3LjQxNCAxNkwyNCA5LjQxNEwyMi41ODYgOEwxNiAxNC41ODZMOS40MTQgOEw4IDkuNDE0TDE0LjU4NiAxNkw4IDIyLjU4Nkw5LjQxNCAyNEwxNiAxNy40MTRMMjIuNTg2IDI0TDI0IDIyLjU4NnoiIC8+PC9zdmc+", "security": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE0IDE2LjU5TDExLjQxIDE0TDEwIDE1LjQxbDQgNGw4LThMMjAuNTkgMTB6IiAvPjxwYXRoIGQ9Im0xNiAzMGwtNi4xNzYtMy4yOTNBMTAuOTggMTAuOTggMCAwIDEgNCAxN1Y0YTIgMiAwIDAgMSAyLTJoMjBhMiAyIDAgMCAxIDIgMnYxM2ExMC45OCAxMC45OCAwIDAgMS01LjgyNCA5LjcwN1pNNiA0djEzYTguOTkgOC45OSAwIDAgMCA0Ljc2NiA3Ljk0MkwxNiAyNy43MzNsNS4yMzQtMi43OUE4Ljk5IDguOTkgMCAwIDAgMjYgMTdWNFoiIC8+PC9zdmc+", "network-3": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTMwIDMwaC04di04aDhabS02LTJoNHYtNGgtNFptLTQtMUg4YTYgNiAwIDAgMSAwLTEyaDJ2Mkg4YTQgNCAwIDAgMCAwIDhoMTJaIiAvPjxwYXRoIGQ9Ik0yMCAyMGgtOHYtOGg4Wm0tNi0yaDR2LTRoLTRaIiAvPjxwYXRoIGQ9Ik0yNCAxN2gtMnYtMmgyYTQgNCAwIDAgMCAwLThIMTJWNWgxMmE2IDYgMCAwIDEgMCAxMm0tMTQtN0gyVjJoOFpNNCA4aDRWNEg0WiIgLz48L3N2Zz4=", "connection-signal": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE1IDEyaDJ2MThoLTJ6bS0zLjY3IDYuMjJhNyA3IDAgMCAxIDAtMTAuNDRsMS4zNCAxLjQ5YTUgNSAwIDAgMCAwIDcuNDZabTkuMzQgMGwtMS4zNC0xLjQ5YTUgNSAwIDAgMCAwLTcuNDZsMS4zNC0xLjQ5YTcgNyAwIDAgMSAwIDEwLjQ0IiAvPjxwYXRoIGQ9Ik04LjQgMjEuOGExMSAxMSAwIDAgMSAwLTE3LjZsMS4yIDEuNmE5IDkgMCAwIDAgMCAxNC40Wm0xNS4yIDBsLTEuMi0xLjZhOSA5IDAgMCAwIDAtMTQuNGwxLjItMS42YTExIDExIDAgMCAxIDAgMTcuNiIgLz48L3N2Zz4=", "data-base": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTI0IDNIOGEyIDIgMCAwIDAtMiAydjIyYTIgMiAwIDAgMCAyIDJoMTZhMiAyIDAgMCAwIDItMlY1YTIgMiAwIDAgMC0yLTJtMCAydjZIOFY1Wk04IDE5di02aDE2djZabTAgOHYtNmgxNnY2WiIgLz48Y2lyY2xlIGN4PSIxMSIgY3k9IjgiIHI9IjEiIC8+PGNpcmNsZSBjeD0iMTEiIGN5PSIxNiIgcj0iMSIgLz48Y2lyY2xlIGN4PSIxMSIgY3k9IjI0IiByPSIxIiAvPjwvc3ZnPg==", "screen": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTI4IDRINGEyIDIgMCAwIDAtMiAydjE2YTIgMiAwIDAgMCAyIDJoOHY0SDh2MmgxNnYtMmgtNHYtNGg4YTIgMiAwIDAgMCAyLTJWNmEyIDIgMCAwIDAtMi0yTTE4IDI4aC00di00aDRabTEwLTZINFY2aDI0WiIgLz48L3N2Zz4=", "settings": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTI3IDE2Ljc2di0xLjUzbDEuOTItMS42OEEyIDIgMCAwIDAgMjkuMyAxMWwtMi4zNi00YTIgMiAwIDAgMC0xLjczLTFhMiAyIDAgMCAwLS42NC4xbC0yLjQzLjgyYTExIDExIDAgMCAwLTEuMzEtLjc1bC0uNTEtMi41MmEyIDIgMCAwIDAtMi0xLjYxaC00LjY4YTIgMiAwIDAgMC0yIDEuNjFsLS41MSAyLjUyYTExLjUgMTEuNSAwIDAgMC0xLjMyLjc1bC0yLjM4LS44NkEyIDIgMCAwIDAgNi43OSA2YTIgMiAwIDAgMC0xLjczIDFMMi43IDExYTIgMiAwIDAgMCAuNDEgMi41MUw1IDE1LjI0djEuNTNsLTEuODkgMS42OEEyIDIgMCAwIDAgMi43IDIxbDIuMzYgNGEyIDIgMCAwIDAgMS43MyAxYTIgMiAwIDAgMCAuNjQtLjFsMi40My0uODJhMTEgMTEgMCAwIDAgMS4zMS43NWwuNTEgMi41MmEyIDIgMCAwIDAgMiAxLjYxaDQuNzJhMiAyIDAgMCAwIDItMS42MWwuNTEtMi41MmExMS41IDExLjUgMCAwIDAgMS4zMi0uNzVsMi40Mi44MmEyIDIgMCAwIDAgLjY0LjFhMiAyIDAgMCAwIDEuNzMtMWwyLjI4LTRhMiAyIDAgMCAwLS40MS0yLjUxWk0yNS4yMSAyNGwtMy40My0xLjE2YTguOSA4LjkgMCAwIDEtMi43MSAxLjU3TDE4LjM2IDI4aC00LjcybC0uNzEtMy41NWE5LjQgOS40IDAgMCAxLTIuNy0xLjU3TDYuNzkgMjRsLTIuMzYtNGwyLjcyLTIuNGE4LjkgOC45IDAgMCAxIDAtMy4xM0w0LjQzIDEybDIuMzYtNGwzLjQzIDEuMTZhOC45IDguOSAwIDAgMSAyLjcxLTEuNTdMMTMuNjQgNGg0LjcybC43MSAzLjU1YTkuNCA5LjQgMCAwIDEgMi43IDEuNTdMMjUuMjEgOGwyLjM2IDRsLTIuNzIgMi40YTguOSA4LjkgMCAwIDEgMCAzLjEzTDI3LjU3IDIwWiIgLz48cGF0aCBkPSJNMTYgMjJhNiA2IDAgMSAxIDYtNmE1Ljk0IDUuOTQgMCAwIDEtNiA2bTAtMTBhMy45MSAzLjkxIDAgMCAwLTQgNGEzLjkxIDMuOTEgMCAwIDAgNCA0YTMuOTEgMy45MSAwIDAgMCA0LTRhMy45MSAzLjkxIDAgMCAwLTQtNCIgLz48L3N2Zz4=", "activity": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTEyIDI5YTEgMSAwIDAgMS0uOTItLjYyTDYuMzMgMTdIMnYtMmg1YTEgMSAwIDAgMSAuOTIuNjJMMTIgMjUuMjhsOC4wNi0yMS42M0ExIDEgMCAwIDEgMjEgM2ExIDEgMCAwIDEgLjkzLjY4TDI1LjcyIDE1SDMwdjJoLTVhMSAxIDAgMCAxLS45NS0uNjhMMjEgN2wtOC4wNiAyMS4zNUExIDEgMCAwIDEgMTIgMjkiIC8+PC9zdmc+", "document": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTI1LjcgOS4zbC03LTdjLS4yLS4yLS40LS4zLS43LS4zSDhjLTEuMSAwLTIgLjktMiAydjI0YzAgMS4xLjkgMiAyIDJoMTZjMS4xIDAgMi0uOSAyLTJWMTBjMC0uMy0uMS0uNS0uMy0uN00xOCA0LjRsNS42IDUuNkgxOHpNMjQgMjhIOFY0aDh2NmMwIDEuMS45IDIgMiAyaDZ6IiAvPjxwYXRoIGQ9Ik0xMCAyMmgxMnYySDEwem0wLTZoMTJ2MkgxMHoiIC8+PC9zdmc+", "download": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTI2IDI0djRINnYtNEg0djRhMiAyIDAgMCAwIDIgMmgyMGEyIDIgMCAwIDAgMi0ydi00Wm0wLTEwbC0xLjQxLTEuNDFMMTcgMjAuMTdWMmgtMnYxOC4xN2wtNy41OS03LjU4TDYgMTRsMTAgMTB6IiAvPjwvc3ZnPg==", "terminal": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTI2IDQuMDFINmEyIDIgMCAwIDAtMiAydjIwYTIgMiAwIDAgMCAyIDJoMjBhMiAyIDAgMCAwIDItMnYtMjBhMiAyIDAgMCAwLTItMm0wIDJ2NEg2di00Wm0tMjAgMjB2LTE0aDIwdjE0WiIgLz48cGF0aCBkPSJtMTAuNzYgMTYuMThsMi44MiAyLjgzbC0yLjgyIDIuODNsMS40MSAxLjQxbDQuMjQtNC4yNGwtNC4yNC00LjI0eiIgLz48L3N2Zz4=", "bare-metal-server": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PGNpcmNsZSBjeD0iOSIgY3k9IjYiIHI9IjEiIC8+PHBhdGggZD0iTTI2IDEwSDZhMiAyIDAgMCAxLTItMlY0YTIgMiAwIDAgMSAyLTJoMjBhMiAyIDAgMCAxIDIgMnY0YTIgMiAwIDAgMS0yIDJNNiA0djRoMjBWNFoiIC8+PGNpcmNsZSBjeD0iOSIgY3k9IjE2IiByPSIxIiAvPjxwYXRoIGQ9Ik0yNiAyMEg2YTIgMiAwIDAgMS0yLTJ2LTRhMiAyIDAgMCAxIDItMmgyMGEyIDIgMCAwIDEgMiAydjRhMiAyIDAgMCAxLTIgMk02IDE0djRoMjB2LTRaIiAvPjxjaXJjbGUgY3g9IjkiIGN5PSIyNiIgcj0iMSIgLz48cGF0aCBkPSJNMjYgMzBINmEyIDIgMCAwIDEtMi0ydi00YTIgMiAwIDAgMSAyLTJoMjBhMiAyIDAgMCAxIDIgMnY0YTIgMiAwIDAgMS0yIDJNNiAyNHY0aDIwdi00WiIgLz48L3N2Zz4=", "earth": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE2IDJhMTQgMTQgMCAxIDAgMTQgMTRBMTQuMDE2IDE0LjAxNiAwIDAgMCAxNiAybTUgMy4xMDZhMTIgMTIgMCAwIDEgMi45MTYgMS44OTlMMjMuNTAzIDhIMjFabS03LjYyMiAyMi41OTdBMTEuOTc2IDExLjk3NiAwIDAgMSA4LjExNiA2Ljk3Nkw5LjQ2NSA5aDMuMzQybC0xLjUgNEg3LjI4bC0xLjM4MiA0LjE0OEw4LjQ2NSAyMWg1bDEuNDMyIDIuMTQ4Wk0xNiAyOGMtLjIwMyAwLS40MDItLjAyLS42MDItLjAzbDEuMzk2LTQuMTlhMS45OSAxLjk5IDAgMCAwLS4yMzMtMS43NDFsLTEuNDMyLTIuMTQ4QTIgMiAwIDAgMCAxMy40NjUgMTloLTMuOTNsLTEuNDMyLTIuMTQ4TDguNzIxIDE1SDExdjJoMnYtMi44MTlsMi45MzYtNy44M2wtMS44NzItLjcwMkwxMy41NTcgN2gtMy4wMjJsLS44MDctMS4yMUExMS44IDExLjggMCAwIDEgMTkgNC4zOTRWOGEyLjAwMyAyLjAwMyAwIDAgMCAyIDJoMi41ODZBMiAyIDAgMCAwIDI1IDkuNDE0bC4xNC0uMTRsLjI4Mi0uNjhBMTIgMTIgMCAwIDEgMjcuMyAxMmgtNC43MDFhMS45OSAxLjk5IDAgMCAwLTEuOTcyIDEuNjY1bC0uNTk3IDMuNDQxYTEuOTkgMS45OSAwIDAgMCAuOTkxIDIuMDg2bDIuMTY1IDEuNDY0bDEuNDU4IDMuNjQ2QTExLjk2IDExLjk2IDAgMCAxIDE2IDI4bTguODE1LTguNjU2TDIyLjEgMTcuNTA5bC0uMS0uMDZsLjU5OS0zLjQ0OWg1LjIyYTExLjc0IDExLjc0IDAgMCAxLTEuNzQ0IDguNDk1WiIgLz48L3N2Zz4=", "search": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTI5IDI3LjU4NmwtNy41NTItNy41NTJhMTEuMDE4IDExLjAxOCAwIDEgMC0xLjQxNCAxLjQxNEwyNy41ODYgMjlaTTQgMTNhOSA5IDAgMSAxIDkgOWE5LjAxIDkuMDEgMCAwIDEtOS05IiAvPjwvc3ZnPg==", "unlink": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0ibTMuNTg0IDUuMDAybDEuNDE0LTEuNDE0bDMuNDE2IDMuNDE0TDcgOC40MTd6bTE5Ljk5NyAxOS45OWwxLjQxNS0xLjQxNGwzLjQxNCAzLjQxNmwtMS40MTQgMS40MTR6TTExIDJoMnY0aC0yem0tOSA5aDR2Mkgyem0yNCA4aDR2MmgtNHptLTcgN2gydjRoLTJ6bS0yLjQyLTQuOTNsLTMuNzEgMy43MmE0IDQgMCAxIDEtNS42Ni01LjY2bDMuNzItMy43Mkw5LjUxIDE0TDUuOCAxNy43MmE2IDYgMCAwIDAtLjA2IDguNTRBNiA2IDAgMCAwIDEwIDI4YTYuMDcgNi4wNyAwIDAgMCA0LjMyLTEuOEwxOCAyMi40OVptLTEuMTctMTAuMTRsMy43Mi0zLjcyYTQgNCAwIDEgMSA1LjY2IDUuNjZsLTMuNzIgMy43MkwyMi40OSAxOGwzLjcxLTMuNzJhNiA2IDAgMCAwIC4wNi04LjU0QTYgNiAwIDAgMCAyMiA0YTYuMDcgNi4wNyAwIDAgMC00LjMyIDEuOEwxNCA5LjUxWiIgLz48L3N2Zz4=", "time": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTE2IDMwYTE0IDE0IDAgMSAxIDE0LTE0YTE0IDE0IDAgMCAxLTE0IDE0bTAtMjZhMTIgMTIgMCAxIDAgMTIgMTJBMTIgMTIgMCAwIDAgMTYgNCIgLz48cGF0aCBkPSJNMjAuNTkgMjJMMTUgMTYuNDFWN2gydjguNThsNSA1LjAxeiIgLz48L3N2Zz4=", "chevron-right": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAzMiAzMiI+PHBhdGggZD0iTTIyIDE2TDEyIDI2bC0xLjQtMS40bDguNi04LjZsLTguNi04LjZMMTIgNnoiIC8+PC9zdmc+"});
const icon=name=>`<span class="glyph" aria-hidden="true" style="--glyph:url('${icons[name]||icons.settings}')"></span>`;
const utility=(label,action,glyph)=>`<button type="button" class="quiet utility" data-action="${action}" aria-label="${label}" title="${label}">${icon(glyph)}</button>`;
root.querySelector('.identity').innerHTML='<span>Local session · admin</span>'+utility('Lock screen — node keeps running','lock','locked');
root.querySelector('footer').innerHTML='<span>anima-home · Anima OS Preview</span><div class="actions">'+utility('Restart computer','restart','renew')+utility('Shut down computer','shutdown','power')+'</div>';

nav.innerHTML='<small>This computer</small>'+names.map(n=>`<button data-page="${n}">${icon(n)}${n}</button>`).join('');
const actionIcons={'sign-in':'user-avatar','signin-complete':'arrow-right','add-computer':'add','switch-account':'user-avatar','overview':'arrow-left','anima':'connection-signal','network':'network-3','close':'close','wifi-toggle':'connection-signal','wifi-password':'connection-signal','guest-password':'connection-signal','wifi-confirm':'checkmark','wifi-details':'settings','network-save':'checkmark','check-hub':'renew','drive-details':'data-base','diagnostics':'activity','report':'document','export':'download','power':'power','restart':'renew','shutdown':'power','power-confirm':'checkmark','lock':'locked','terminal':'terminal','keep-rotation':'checkmark'};
const button=(label,action,primary=false)=>`<button type="button" data-action="${action}" class="${primary?'primary':''}">${icon(/Unlock/.test(label)?'password':label==='Done'?'checkmark':actionIcons[action]||'arrow-right')}<span>${label}</span></button>`;
const rowIcon=name=>name==='Sign in'?'user-avatar':name==='Ready'?'checkmark':/Wi-Fi|network|Wireless/.test(name)?'connection-signal':/Storage|drive|Filesystem|Capacity|Storage/.test(name)?'data-base':/Account|Signed in/.test(name)?'user-avatar':/Workspace|workspace|Anima/.test(name)?'network-3':/Computer|computer|Device/.test(name)?'bare-metal-server':/Security|Encryption/.test(name)?'security':/Internet|Address/.test(name)?'earth':/report/.test(name)?'document':'activity';
const row=(name,detail,right)=>`<div class="row"><div class="row-title">${icon(rowIcon(name))}<div>${name}<small>${detail}</small></div></div><div class="value">${right}</div></div>`;

const head=(title,body)=>`<h1>${title}</h1><p>${body}</p>`;
let state=null,available=[],displays=[],signedIn=false,email='',locked=true,pending=0,bridge=null,poll=null,powerAction='',selected=null;
const waiting=new Map();
const esc=value=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const size=value=>(value/1024**3).toFixed(1)+' GB';
function draw(){
 nav.querySelectorAll('button').forEach(b=>b.setAttribute('aria-current',b.dataset.page===page?'page':'false'));
 const screens={
 Overview:()=>head('Your Anima server', 'Local controls stay available even when your hub is offline.')+`<section class="hero"><img src="hero.png" alt=""><h2>${esc(state?.hostname||'This computer')}</h2><p>${state?.enrolled?'Daemon enrollment is saved on this machine.':'Connect this computer to your Anima account.'}</p>${button(state?.enrolled?'View connection':'Sign in to Anima','anima',true)}</section><div class="section"><h2>At a glance</h2>${row('Anima connection','Local monitoring service',esc(state?.daemon||'Checking'))}${row('Network','NetworkManager state',esc(state?.network||'Checking'))}${row('Storage','System and persistent data',state?size(state.storage.used)+' of '+size(state.storage.total):'Checking')}</div>`,
 Network:()=>head('Network','Connect this computer and inspect its actual network state.')+(state?.radio==='disabled'?'<div class=notice>Wi-Fi is switched off. '+button('Turn Wi-Fi on','radio-on')+'</div>':'')+(state?.supplicant==='failed'?'<div class=notice>The Wi-Fi system service failed to start. Open Recovery for diagnostics.</div>':'')+(state?.devices||[]).filter(d=>['wifi','ethernet'].includes(d.type)).map(d=>row(esc(d.device),esc(d.connection),esc(d.state))).join('')+`<div class="section"><div class="topline"><h2>Available Wi-Fi networks</h2>${button('Scan networks','scan')}</div>${available.map((n,i)=>row(esc(n.ssid||'Hidden network'),esc(n.device+' · '+n.signal+'% · '+n.security),button('Connect','wifi-'+i))).join('')||'<p>Scan to find networks. Ethernet connects automatically when a usable cable is present.</p>'}</div>`,
 Anima:()=>state?.enrolled?head('Connected to Anima','The daemon enrollment is saved. Service state below is local; it does not prove the hub is reachable.')+row('Computer',esc(state.hostname),esc(state.daemon))+`<div class="notice">Locking this screen keeps the daemon and its private link running.</div>`:signedIn?head('Add this computer','Confirm this machine. Anima will connect its private link and enroll the monitoring daemon automatically.')+row('Signed in',esc(email),esc(state?.hostname))+`<div class="notice">This authorizes enrollment with your account’s hub. It does not open remote shell access or grant new workload permissions.</div>${button('Add this computer','add-computer',true)}`:head('Sign in to Anima','Sign in securely, then add this computer. No enrollment code to copy.')+`<section class="hero"><img src="hero.png" alt="">${button('Sign in to Anima','sign-in',true)}<p class="hint" style="margin-top:16px">Secure sign-in with WorkOS</p></section><div class="section">${row('Sign in','Continue in the secure sign-in page','')}${row('Add this computer','Confirm your account and this machine','')}${row('Ready','Anima enrolls the daemon automatically','')}</div>`,
 Storage:()=>head('Storage','Actual disks and persistent capacity on this machine.')+(state?`<div class="numbers"><div><div class="overline">Used</div><div class="number">${size(state.storage.used)}</div></div><div><div class="overline">Available</div><div class="number">${size(state.storage.total-state.storage.used)}</div></div><div><div class="overline">Filesystem</div><div class="number">Btrfs</div></div></div>`:'')+(state?.drives||[]).filter(d=>d.type==='disk').map(d=>row(esc(d.model||d.name),esc('/dev/'+d.name),size(d.size))).join('')+'<div class="notice">The OS root is read-only. Workload data is stored separately in persistent storage.</div>',
 Display:()=>head('Display','Keep the local screen readable and correctly oriented.')+`<form id="display-form"><label>Display<select name="output">${displays.map(d=>`<option value="${esc(d.name)}">${esc(d.name)}</option>`).join('')}</select></label><label>Orientation<select name="rotation"><option value="normal">Landscape</option><option value="left">Portrait left</option><option value="right">Portrait right</option><option value="inverted">Landscape flipped</option></select></label><button class="primary" ${displays.length?'':'disabled'}>${icon('checkmark')}Apply display settings</button></form><div class="notice">Changes revert after 15 seconds unless confirmed. Confirmed changes are saved for future boots.</div><details><summary>Motion and screen behaviour</summary><label style="margin-top:16px">Interface motion<select id="motion-choice"><option value="system" ${motionChoice==='system'?'selected':''}>Follow system</option><option value="reduced" ${motionChoice==='reduced'?'selected':''}>Reduced motion</option></select></label><p>Locking the screen does not suspend the server.</p></details>`,
 Recovery:()=>head('Recovery','Inspect system and network state without opening a terminal.')+row('System diagnostics','Radio, network and monitoring checks',button('Run checks','diagnostics'))+'<div class="notice">For advanced recovery, press Ctrl+Alt+F2 and log in as admin. Return to the graphical session with Ctrl+Alt+F7. OS rollback remains available from the boot menu.</div>'};
 showPage(screens[page]());
 main.querySelectorAll('.page-content summary').forEach(el=>{el.innerHTML=icon('settings')+'<span>'+el.textContent+'</span>';});
}
function close(immediate=false){
 clearInterval(timer);timer=null;
 if(overlay.hidden||closing)return;
 closing=true;const version=++dialogVersion;
 const finish=()=>{
  if(version!==dialogVersion)return;
  overlay.hidden=true;overlay.innerHTML='';closing=false;
  root.querySelector('.shell').removeAttribute('data-locked');
  backgroundInert(false);
  if(returnFocus?.isConnected)returnFocus.focus();
 };
 if(immediate||reduced()){finish();return;}
 animate(overlay,[{opacity:getComputedStyle(overlay).opacity},{opacity:0}],role.fast).then(finish);
}
function modal(title,body,actions){
 ++dialogVersion;closing=false;
 if(overlay.hidden)returnFocus=document.activeElement;
 overlay.getAnimations({subtree:true}).forEach(animation=>animation.cancel());
 clearInterval(timer);timer=null;
 overlay.innerHTML=`<section class="dialog" role="dialog" aria-modal="true" aria-label="${title}"><h2>${icon(/locked|sign-in/i.test(title)?'locked':/Restart|Shut down|Power/.test(title)?'power':/Wi-Fi|network/i.test(title)?'connection-signal':'settings')}${title}</h2>${body}<div class="actions">${actions}</div></section>`;
 overlay.hidden=false;backgroundInert(true);
 const locked=title==='Local session locked';
 root.querySelector('.shell').toggleAttribute('data-locked',locked);
 if(!locked){
  animate(overlay,[{opacity:0},{opacity:1}],role.fast);
  animate(overlay.querySelector('.dialog'),[{transform:'translateY(6px)',opacity:0},{transform:'translateY(0)',opacity:1}]);
 }
 overlay.querySelector('input,button')?.focus();
}
function call(op,args={}){
 if(!bridge)return Promise.reject(Error('The local service is starting.'));
 const id=op==='unlock'?'unlock':crypto.randomUUID();
 if(waiting.has(id))return Promise.reject(Error('Wait for the current request.'));
 pending++;
 return new Promise((resolve,reject)=>{waiting.set(id,{resolve,reject});bridge.request(JSON.stringify({id,op,args}));});
}
function errorMessage(error){
 let message=overlay.querySelector('.message');
 if(overlay.hidden){modal('Unable to finish','<p class="message"></p>',button('Close','close'));message=overlay.querySelector('.message');}
 if(!message){message=document.createElement('p');message.className='message';overlay.querySelector('.dialog').append(message);}
 message.textContent=error.message;
 if(!locked&&!overlay.querySelector('[data-action=close]'))overlay.querySelector('.actions').innerHTML=button('Close','close');
}
function lock(){
 locked=true;signedIn=false;email='';state=null;available=[];
 clearInterval(poll);poll=null;
 close(true);
 modal('Local session locked','<p>Your node stays awake. Apps, monitoring and authorized remote access continue running.</p><form id="unlock-form"><label>Local admin password<input type="password" name="password" autocomplete="current-password" required></label><button class="primary" type="submit">'+icon('password')+'Unlock</button></form>','');
}
async function refresh(redraw=true){state=await call('status');if(redraw&&!locked)draw();}
async function go(n){page=n;close(true);draw();if(n==='Display'){displays=await call('displays');draw();}}
async function busy(op,args,label){
 modal(label,'<p>Please wait. This is a real operation on this computer.</p>','');
 try{const result=await call(op,args);close(true);return result;}catch(error){errorMessage(error);throw error;}
}
root.addEventListener('click',async e=>{
 const b=e.target.closest('button');if(!b)return;
 const a=b.dataset.action;
 if(a==='lock'){bridge.lock();return;}
 if(locked)return;
 if(pending)return;
 try{
  if(b.dataset.page){await go(b.dataset.page);return;}
  if(a==='close'){close();return;}
  if(['overview','network','anima'].includes(a)){await go(a[0].toUpperCase()+a.slice(1));return;}
  if(a==='scan'){available=await busy('scan',{},'Scanning Wi-Fi');draw();}
  if(a==='radio-on'){await busy('radio-on',{},'Enabling Wi-Fi');state=await call('status');await go('Network');return;}
  if(a?.startsWith('wifi-')){selected=available[Number(a.slice(5))];if(!selected)return;modal('Connect to Wi-Fi',`<p>${esc(selected.ssid)} · ${esc(selected.device)}</p><form id="wifi-form"><label>Network password<input name="password" type="password" autocomplete="off"></label><p class="hint">Leave blank for an open network. Enterprise Wi-Fi requires advanced recovery configuration.</p><button class="primary">${icon('checkmark')}Connect</button></form>`,button('Cancel','close'));}
  if(a==='sign-in'){
   await busy('sign-in',{},'Opening secure sign-in');
   modal('Continue in your browser','<p>Complete sign-in in the secure browser. Anima will return here automatically.</p>',button('Cancel sign-in','cancel-sign-in'));
   poll=setInterval(async()=>{if(pending||locked)return;try{const result=await call('sign-in-poll');if(result.signedIn){clearInterval(poll);poll=null;signedIn=true;email=result.email;close(true);draw();}}catch(error){clearInterval(poll);poll=null;errorMessage(error);}},1000);
  }
  if(a==='cancel-sign-in'){clearInterval(poll);poll=null;await call('cancel-sign-in');close();}
  if(a==='add-computer'){await busy('enroll',{},'Adding this computer');signedIn=false;await refresh();}
  if(a==='diagnostics'){const result=await busy('diagnostics',{},'Checking the system');modal('System diagnostics',Object.entries(result).map(([k,v])=>row(esc(k),'',esc(v))).join(''),button('Done','close',true));}
  if(a==='restart'||a==='shutdown'){powerAction=a;modal(a==='restart'?'Restart this computer?':'Shut down this computer?','<p>This stops services and disconnects remote access. Running work may be interrupted.</p>',button('Cancel','close')+button(a==='restart'?'Restart':'Shut down','power-confirm',true));}
  if(a==='power-confirm'){await busy(powerAction,{confirmed:true},'Stopping this computer');}
  if(a==='keep-rotation'){await call('keep-rotation');close();}
  if(a==='revert'){await call('revert');close();}
 }catch(error){if(a==='add-computer'){signedIn=false;email='';draw();}errorMessage(error);}
});
root.addEventListener('submit',async e=>{
 e.preventDefault();const form=e.target;if(pending)return;
 try{
  if(form.id==='unlock-form'){
   const password=form.elements.password.value;form.elements.password.value='';
   await call('unlock',{password});locked=false;close(true);await refresh();return;
  }
  if(locked)return;
  if(form.id==='wifi-form'){
   const password=form.elements.password.value;form.elements.password.value='';
   await busy('connect',{device:selected.device,bssid:selected.bssid,password},'Connecting to Wi-Fi');await refresh();
  }
  if(form.id==='display-form'){
   await call('rotate',{output:form.elements.output.value,rotation:form.elements.rotation.value});
   const deadline=Date.now()+15000;
   modal('Keep these display settings?','<p id="revert-count">Reverting in 15 seconds.</p>',button('Revert','revert')+button('Keep settings','keep-rotation',true));
   timer=setInterval(()=>{const left=Math.max(0,Math.ceil((deadline-Date.now())/1000));if(!left){close();return;}overlay.querySelector('#revert-count').textContent=`Reverting in ${left} seconds.`;},200);
  }
 }catch(error){errorMessage(error);}
});
overlay.addEventListener('keydown',e=>{
 if(e.key==='Escape'&&!locked&&!pending&&!overlay.querySelector('#revert-count'))close();
 if(e.key==='Tab'){const controls=[...overlay.querySelectorAll('button,input,select')];const first=controls[0],last=controls.at(-1);if(e.shiftKey&&document.activeElement===first){e.preventDefault();last?.focus();}else if(!e.shiftKey&&document.activeElement===last){e.preventDefault();first?.focus();}}
});
root.addEventListener('change',e=>{if(e.target.id==='motion-choice'){motionChoice=e.target.value;localStorage.setItem('motion',motionChoice);syncMotion();}});
root.addEventListener('toggle',e=>{if(e.target.tagName==='DETAILS'&&e.target.open)for(const child of e.target.children)if(child.tagName!=='SUMMARY')animate(child,[{opacity:0},{opacity:1}]);},true);
motionChoice=localStorage.getItem('motion')==='reduced'?'reduced':'system';syncMotion();draw();lock();
new QWebChannel(qt.webChannelTransport,channel=>{
 bridge=channel.objects.console;
 bridge.answer.connect(text=>{const answer=JSON.parse(text),task=waiting.get(answer.id);if(!task)return;waiting.delete(answer.id);pending=Math.max(0,pending-1);answer.ok?task.resolve(answer.value):task.reject(Error(answer.error));});
 bridge.locked.connect(()=>{for(const task of waiting.values())task.reject(Error('The local session is locked.'));waiting.clear();pending=0;if(!locked)lock();});
 setInterval(()=>{if(!locked&&!pending&&overlay.hidden)refresh(page==='Overview').catch(()=>{});},10000);
});

})();
