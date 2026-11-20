let localReadings=null;
function updateTelemetry(value){localReadings=value;renderTelemetry();}
function renderTelemetry(){
 if(!localReadings)return;
 const batteries=document.querySelector('#battery-status');
 batteries.replaceChildren();
 for(const battery of localReadings.batteries){
  const badge=document.createElement('span');
  const percentage=battery.percent;
  const label=`${battery.name}: ${percentage===null?'charge unknown':Math.round(percentage)+'%'} · ${battery.state}`;
  badge.className='battery';badge.title=label;badge.setAttribute('aria-label',label);badge.setAttribute('role','img');
  badge.dataset.low=percentage!==null&&percentage<=15&&battery.state==='Discharging';
  badge.innerHTML=`<svg viewBox="0 0 32 20" aria-hidden="true"><rect x="1" y="3" width="26" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M29 7v6" stroke="currentColor" stroke-width="2"/>${percentage===null?'<text x="10" y="14" fill="currentColor" font-size="12">?</text>':`<rect x="4" y="6" width="${20*percentage/100}" height="8" rx="1" fill="currentColor"/>`}${battery.state==='Charging'?'<path d="m17 1-7 10h5l-1 8 8-11h-6z" fill="#1B1D21" stroke="currentColor"/>':''}</svg>`;
  batteries.append(badge);
 }
 const panel=document.querySelector('#telemetry-readings');
 if(!panel)return;
 const r=localReadings;
 const metrics=[['CPU',r.cpu],['Memory',r.memoryTotal&&r.memoryUsed!==null?100*r.memoryUsed/r.memoryTotal:null]];
 panel.innerHTML=metrics.map(([label,value])=>`<div class="metric"><span>${label}</span><strong>${value===null?'Collecting…':Math.round(value)+'%'}</strong><meter min="0" max="100" value="${value??0}" aria-label="${label} usage" ${value===null?'hidden':''}></meter></div>`).join('');
 const uptime=document.createElement('p');uptime.className='hint';
 uptime.textContent=r.uptime===null?'Uptime unavailable':`Uptime · ${Math.floor(Number(r.uptime)/3600)}h ${Math.floor(Number(r.uptime)%3600/60)}m · Local readings, refreshed every 5 seconds`;
 panel.append(uptime);
}
