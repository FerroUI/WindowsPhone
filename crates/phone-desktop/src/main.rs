use ferroui_controls::{AppBuilder,Application,Control,Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use phone_domain::PhoneData;
use phone_state::{HostServices,PhoneViewModel};
use std::{path::PathBuf,rc::Rc};

fn data_path()->PathBuf {
    std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
        .or_else(||std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(||std::env::var_os("HOME").map(|p|PathBuf::from(p).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir).join("ferroui-windows-phone/data.json")
}
fn main(){
    AppBuilder::configure::<Application>().use_platform_detect().with_inter_font().start(|app,_|{
        phone_controls::install_theme(app);
        let path=data_path();
        let data=std::fs::read_to_string(&path).ok().and_then(|json|PhoneData::from_json(&json).ok()).unwrap_or_else(PhoneData::demo);
        let save_path=path.clone();
        let vm=PhoneViewModel::new(data,Rc::new(move |data|{
            if let (Some(parent),Ok(json))=(save_path.parent(),data.to_json()){
                let result=std::fs::create_dir_all(parent).and_then(|_|std::fs::write(save_path.with_extension("tmp"),json)).and_then(|_|std::fs::rename(save_path.with_extension("tmp"),&save_path));
                if let Err(error)=result{eprintln!("Could not save Windows Phone data: {error}");}
            }
        }));
        vm.set_clock(chrono::Local::now().timestamp());
        let host_vm=Rc::downgrade(&vm);
        vm.set_services(HostServices{request:Rc::new(move |request|{
            if let Some(vm)=host_vm.upgrade(){vm.set_status(format!("This service is available in the browser edition. Request: {request:?}"));}
        })});
        let window=Window::new();window.set_title(Some("Windows Phone".into()));window.set_width(430.0);window.set_height(860.0);window.set_min_width(320.0);window.set_min_height(560.0);
        vm.set_viewport(430.0,860.0);window.set_content(Some(Control::boxed(&phone_ui::create_view(vm.clone()))));
        let weak_window=window.downgrade();let weak_vm=Rc::downgrade(&vm);
        window.layout_updated(move ||if let (Some(window),Some(vm))=(weak_window.upgrade(),weak_vm.upgrade()){let size=window.client_size();vm.set_viewport(size.width,size.height);}).detach();
        app.run_window(window);
    },&std::env::args().collect::<Vec<_>>());
}
