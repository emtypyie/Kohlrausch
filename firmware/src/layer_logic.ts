export function nextLayer(layer:1|2|3|4):1|2|3|4{return layer===4?1:(layer+1) as 1|2|3|4}
export function createLayerController(start:1|2|3|4=1){let active=start;return{get active(){return active},pressLyr(){active=nextLayer(active);return active}}}
